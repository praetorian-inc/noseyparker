use anyhow::{bail, Context, Result};
use bstr::BString;
use fixedbitset::FixedBitSet;
use gix::hashtable::{hash_map, HashMap};
use gix::objs::tree::EntryKind;
use gix::prelude::*;
use gix::{object::Kind, ObjectId, OdbHandle};
use petgraph::graph::{DiGraph, EdgeIndex, IndexType, NodeIndex};
use petgraph::prelude::*;
use petgraph::visit::Visitable;
use roaring::RoaringBitmap;
use smallvec::SmallVec;
use std::collections::BinaryHeap;
use std::time::Instant;
use tracing::{debug, error, error_span, warn};

use crate::bstring_table::{BStringTable, SymbolType};
use crate::{unwrap_ok_or_continue, unwrap_some_or_continue};

type Symbol = crate::bstring_table::Symbol<u32>;

/// A newtype for commit graph indexes, to prevent mixing up indexes from different types of graphs
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Default, Debug)]
pub(crate) struct CommitGraphIdx(NodeIndex);

/// Boilerplate instance for the index newtype
unsafe impl IndexType for CommitGraphIdx {
    #[inline(always)]
    fn new(x: usize) -> Self {
        Self(NodeIndex::new(x))
    }
    #[inline(always)]
    fn index(&self) -> usize {
        self.0.index()
    }
    #[inline(always)]
    fn max() -> Self {
        Self(<NodeIndex as IndexType>::max())
    }
}

type CommitNodeIdx = NodeIndex<CommitGraphIdx>;
type CommitEdgeIdx = EdgeIndex<CommitGraphIdx>;

/// A newtype wrapper for a u32, to map to gix::ObjectId to use as an index in other array-based
/// data structures.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Default, Debug)]
pub(crate) struct ObjectIdx(u32);

impl ObjectIdx {
    pub(crate) fn new(x: usize) -> Self {
        Self(x.try_into().unwrap())
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CommitMetadata {
    pub(crate) oid: ObjectId,
    pub(crate) tree_idx: Option<ObjectIdx>,
}

/// A compact set of git objects, denoted via `ObjectIdx`
#[derive(Clone, Debug, Default)]
struct SeenObjectSet {
    seen_trees: RoaringBitmap,
    seen_blobs: RoaringBitmap,
}

impl SeenObjectSet {
    pub(crate) fn new() -> Self {
        SeenObjectSet {
            seen_trees: RoaringBitmap::new(),
            seen_blobs: RoaringBitmap::new(),
        }
    }

    /// Returns whether the value was absent from the set
    fn insert(set: &mut RoaringBitmap, idx: ObjectIdx) -> Result<bool> {
        let idx = idx
            .as_usize()
            .try_into()
            .context("index should be representable with a u32")?;
        Ok(set.insert(idx))
    }

    fn contains(set: &RoaringBitmap, idx: ObjectIdx) -> Result<bool> {
        let idx = idx
            .as_usize()
            .try_into()
            .context("index should be representable with a u32")?;
        Ok(set.contains(idx))
    }

    /// Returns whether the value was absent from the set
    pub(crate) fn insert_tree(&mut self, idx: ObjectIdx) -> Result<bool> {
        Self::insert(&mut self.seen_trees, idx)
    }

    /// Returns whether the value was absent from the set
    pub(crate) fn insert_blob(&mut self, idx: ObjectIdx) -> Result<bool> {
        Self::insert(&mut self.seen_blobs, idx)
    }

    pub(crate) fn contains_blob(&self, idx: ObjectIdx) -> Result<bool> {
        Self::contains(&self.seen_blobs, idx)
    }

    pub(crate) fn union_update(&mut self, other: &Self) {
        self.seen_blobs |= &other.seen_blobs;
        self.seen_trees |= &other.seen_trees;
    }
}

struct ObjectIdBimap {
    oid_to_idx: HashMap<ObjectId, ObjectIdx>,
    idx_to_oid: Vec<ObjectId>,
}

impl ObjectIdBimap {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            oid_to_idx: HashMap::with_capacity_and_hasher(capacity, Default::default()),
            idx_to_oid: Vec::with_capacity(capacity),
        }
    }

    fn insert(&mut self, oid: ObjectId) {
        match self.oid_to_idx.entry(oid) {
            gix::hashtable::hash_map::Entry::Occupied(_e) => {
                // warn!("object {} seen multiple times", e.key());
            }
            gix::hashtable::hash_map::Entry::Vacant(e) => {
                let idx = ObjectIdx::new(self.idx_to_oid.len());
                self.idx_to_oid.push(*e.key());
                e.insert(idx);
            }
        }
    }

    fn get_oid(&self, idx: ObjectIdx) -> Option<&gix::oid> {
        self.idx_to_oid.get(idx.as_usize()).map(|v| v.as_ref())
    }

    fn get_idx(&self, oid: &gix::oid) -> Option<ObjectIdx> {
        self.oid_to_idx.get(oid).copied()
    }

    fn len(&self) -> usize {
        self.idx_to_oid.len()
    }
}

// Some types and data structures for recursively enumerating tree objects
type Symbols = SmallVec<[Symbol; 6]>;
type TreeWorklistItem = (Symbols, ObjectId);
type TreeWorklist = Vec<TreeWorklistItem>;

/// An in-memory index that organizes various objects within a Git repository.
///
/// - It associates a u32-based ID with each object
/// - It partitions object IDs according to object type (commit, blob, tree, tag)
pub(crate) struct RepositoryIndex {
    trees: ObjectIdBimap,
    commits: ObjectIdBimap,
    blobs: ObjectIdBimap,
    tags: ObjectIdBimap,
}

impl RepositoryIndex {
    pub(crate) fn new(odb: &OdbHandle) -> Result<Self> {
        use gix::odb::store::iter::Ordering;
        use gix::prelude::*;

        // Get object count to allow for exact index allocation size
        // Use fastest gix ordering mode
        let mut num_tags = 0;
        let mut num_trees = 0;
        let mut num_blobs = 0;
        let mut num_commits = 0;

        for oid in odb
            .iter()
            .context("Failed to iterate object database")?
            .with_ordering(Ordering::PackLexicographicalThenLooseLexicographical)
        {
            let oid = unwrap_ok_or_continue!(oid, |e| { error!("Failed to read object id: {e}") });
            let hdr = unwrap_ok_or_continue!(odb.header(oid), |e| {
                error!("Failed to read object header for {oid}: {e}")
            });
            match hdr.kind() {
                Kind::Tree => num_trees += 1,
                Kind::Blob => num_blobs += 1,
                Kind::Commit => num_commits += 1,
                Kind::Tag => num_tags += 1,
            }
        }

        // Allocate indexes exactly to the size needed
        let mut trees = ObjectIdBimap::with_capacity(num_trees);
        let mut commits = ObjectIdBimap::with_capacity(num_commits);
        let mut blobs = ObjectIdBimap::with_capacity(num_blobs);
        let mut tags = ObjectIdBimap::with_capacity(num_tags);

        // Now build in-memory index
        // Use slower gix ordering mode, but one that puts objects in a possibly more efficient
        // order for reading
        for oid in odb
            .iter()
            .context("Failed to iterate object database")?
            .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
        {
            let oid = unwrap_ok_or_continue!(oid, |e| { error!("Failed to read object id: {e}") });
            let hdr = unwrap_ok_or_continue!(odb.header(oid), |e| {
                error!("Failed to read object header for {oid}: {e}")
            });
            match hdr.kind() {
                Kind::Tree => trees.insert(oid),
                Kind::Blob => blobs.insert(oid),
                Kind::Commit => commits.insert(oid),
                Kind::Tag => tags.insert(oid),
            }
        }

        Ok(Self {
            trees,
            commits,
            blobs,
            tags,
        })
    }

    pub(crate) fn num_commits(&self) -> usize {
        self.commits.len()
    }

    pub(crate) fn num_blobs(&self) -> usize {
        self.blobs.len()
    }

    pub(crate) fn num_trees(&self) -> usize {
        self.trees.len()
    }

    pub(crate) fn num_tags(&self) -> usize {
        self.tags.len()
    }

    pub(crate) fn num_objects(&self) -> usize {
        self.num_commits() + self.num_blobs() + self.num_tags() + self.num_trees()
    }

    pub(crate) fn get_tree_oid(&self, idx: ObjectIdx) -> Option<&gix::oid> {
        self.trees.get_oid(idx)
    }

    pub(crate) fn get_tree_index(&self, oid: &gix::oid) -> Option<ObjectIdx> {
        self.trees.get_idx(oid)
    }

    pub(crate) fn get_blob_index(&self, oid: &gix::oid) -> Option<ObjectIdx> {
        self.blobs.get_idx(oid)
    }

    pub(crate) fn into_blobs(self) -> Vec<ObjectId> {
        self.blobs.idx_to_oid
    }

    pub(crate) fn commits(&self) -> &[ObjectId] {
        self.commits.idx_to_oid.as_slice()
    }
}

/// A graph of metadata in a Git repository
///
/// This is an in-memory graph of the inverted Git commit graph, i.e., each commit node has an
/// outgoing edge to each child commit. This is backward from how Git natively stores the commit
/// information.
///
/// This is also an in-memory graph of the trees and blobs in Git.
/// Each tree object is a node that has an outgoing edge to each of its immediate children, labeled
/// with the name of that child.
pub(crate) struct GitMetadataGraph {
    commit_oid_to_node_idx: HashMap<ObjectId, CommitNodeIdx>,
    commits: DiGraph<CommitMetadata, (), CommitGraphIdx>,
}

impl GitMetadataGraph {
    /// Create a new commit graph with the given capacity.
    pub(crate) fn with_capacity(num_commits: usize) -> Self {
        // use 2x the number of commits, assuming that most commits have a single parent commit,
        // except merges, which usually have 2
        let commit_edges_capacity = num_commits * 2;

        Self {
            commit_oid_to_node_idx: HashMap::with_capacity_and_hasher(
                num_commits,
                Default::default(),
            ),
            commits: DiGraph::with_capacity(num_commits, commit_edges_capacity),
        }
    }

    /// Get the commit metadata for the given graph node index.
    ///
    /// Panics if the given graph node index is not valid for this graph.
    #[inline]
    pub(crate) fn get_commit_metadata(&self, idx: CommitNodeIdx) -> &CommitMetadata {
        self.commits
            .node_weight(idx)
            .expect("commit graph node index should be valid")
    }

    /// Get the index of the graph node for the given commit, creating it if needed.
    ///
    /// If a node already exists for the given commit and `tree_idx` is given, the node's metadata
    /// is updated with the given value.
    pub(crate) fn get_commit_idx(
        &mut self,
        oid: ObjectId,
        tree_idx: Option<ObjectIdx>,
    ) -> CommitNodeIdx {
        match self.commit_oid_to_node_idx.entry(oid) {
            hash_map::Entry::Occupied(e) => {
                let idx = *e.get();
                if tree_idx.is_some() {
                    let md = self
                        .commits
                        .node_weight_mut(idx)
                        .expect("a commit graph node should exist for given index");
                    md.tree_idx = tree_idx;
                }
                idx
            }
            hash_map::Entry::Vacant(e) => {
                *e.insert(self.commits.add_node(CommitMetadata { oid, tree_idx }))
            }
        }
    }

    /// Add a new edge between two commits, returning its index.
    ///
    /// NOTE: If an edge already exists between the two commits, a parallel edge is added.
    pub(crate) fn add_commit_edge(
        &mut self,
        parent_idx: CommitNodeIdx,
        child_idx: CommitNodeIdx,
    ) -> CommitEdgeIdx {
        // For alternative behavior that doesn't add parallel edges, use
        // `self.commits.update_edge(parent_idx, child_idx, ())`.
        self.commits.add_edge(parent_idx, child_idx, ())
    }
}

pub(crate) type IntroducedBlobs = SmallVec<[(ObjectId, BString); 4]>;

pub(crate) struct CommitBlobMetadata {
    /// index of the commit this entry applies to
    pub(crate) commit_oid: ObjectId,

    /// set of introduced blobs and path names
    pub(crate) introduced_blobs: IntroducedBlobs,
}

impl GitMetadataGraph {
    pub(crate) fn get_repo_metadata(
        self,
        repo_index: &RepositoryIndex,
        repo: &gix::Repository,
    ) -> Result<Vec<CommitBlobMetadata>> {
        let _span =
            error_span!("get_repo_metadata", path = repo.path().display().to_string()).entered();

        let t1 = Instant::now();
        let cg = &self.commits;
        let num_commits = cg.node_count();

        // An adapatation of Kahn's topological sorting algorithm, to visit the commit nodes in
        // topological order: <https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm>
        // This algorithm naturally mantains a frontier of still-to-expand nodes.
        //
        // We attach to each node in the frontier a set of seen blobs and seen trees in the
        // traversal up to that point.

        // A mapping of graph index of a commit to the set of seen trees and blobs.
        //
        // There is one such set for _each_ commit, though at runtime, not all of these seen sets
        // will be "live" (those will have `None` values).
        //
        // XXX: this could be merged with the `commit_worklist` priority queue with a suitable data
        // structure (one allowing O(1) updates of items in the queue)
        let mut seen_sets: Vec<Option<SeenObjectSet>> = vec![None; num_commits];

        // A mapping of graph index of a commit to the set of blobs introduced by that commit
        let mut blobs_introduced: Vec<IntroducedBlobs> = vec![IntroducedBlobs::new(); num_commits];

        // NOTE: petgraph comes with a pre-built data type for keeping track of visited nodes,
        // but has no such thing for keeping track of visited edges, so we make our own.
        let mut visited_commit_edges = FixedBitSet::with_capacity(cg.edge_count());

        // Keep track of which commit nodes we have seen: needed to avoid re-visiting nodes in the
        // rare present of parallel (i.e., multiple) edges between two commits.
        let mut visited_commits = cg.visit_map();

        // We use an ordered queue for the worklist instead of a deque or simple vector.
        // This queue is ordered by ascending commit node out-degree: the commit with the smallest
        // out-degree is popped first.
        //
        // Why? Performing a topological traversal of the commit graph in this order instead is
        // noticably better in terms of memory usage than FIFO order, and drastically better than
        // LIFO order: fewer "seen sets" need to be simultaneously maintained.
        //
        // In the case of CPython, with some 250k commits and 1.3M blobs and trees, I saw the
        // following maximum number of live seen sets:
        //
        // - LIFO: 20.5k
        // - FIFO: 1.5k
        // - Smallest out-degree first: 888
        type OutDegree = std::cmp::Reverse<u32>;

        let commit_out_degree = |idx: CommitNodeIdx| -> Result<OutDegree> {
            let count = cg
                .neighbors_directed(idx, Outgoing)
                .count()
                .try_into()
                .context("out-degree should be representable with a u32")?;
            Ok(std::cmp::Reverse(count))
        };

        // A table for interned bytestrings; used to represent filename path fragments, drastically
        // reducing peak memory use
        let mut symbols = BStringTable::with_capacity(32 * 1024, 1024 * 1024);

        // A queue of commit graph node indexes, ordered by minimum out-degree.
        // Invariant: each entry commit has no unprocessed parent commits
        let mut commit_worklist =
            BinaryHeap::<(OutDegree, CommitNodeIdx)>::with_capacity(num_commits);

        // Initialize with commit nodes that have no parents
        for root_idx in cg
            .node_indices()
            .filter(|idx| cg.neighbors_directed(*idx, Incoming).count() == 0)
        {
            let out_degree = commit_out_degree(root_idx)?;
            commit_worklist.push((out_degree, root_idx));
            seen_sets[root_idx.index()] = Some(SeenObjectSet::new());
        }

        // A worklist of tree objects (and no other type) to be traversed
        let mut tree_worklist = TreeWorklist::with_capacity(32 * 1024);
        let mut tree_buf = Vec::with_capacity(1024 * 1024);
        // A scratch buffer for new blobs encountered while traversing a tree
        let mut blobs_encountered = Vec::with_capacity(16 * 1024);

        // various counters for statistics
        let mut max_frontier_size = 0; // max value of size of `commit_worklist`
        let mut num_blobs_introduced = 0; // total number of blobs introduced in commits
        let mut num_trees_introduced = 0; // total number of trees introduced in commits
        let mut num_commits_visited = 0; // total number of commits visited

        let mut num_live_seen_sets = commit_worklist.len(); // current number of live seen sets
        let mut max_live_seen_sets = num_live_seen_sets; // max value of `num_live_seen_sets`

        while let Some((_out_degree, commit_idx)) = commit_worklist.pop() {
            let commit_index = commit_idx.index();
            if visited_commits.put(commit_index) {
                warn!("found duplicate commit node {commit_index}");
                continue;
            }

            let introduced = &mut blobs_introduced[commit_index];

            let mut seen = seen_sets[commit_index]
                .take()
                .expect("should have a seen set");
            assert!(num_live_seen_sets > 0);
            num_live_seen_sets -= 1;

            // Update stats
            num_commits_visited += 1;
            max_frontier_size = max_frontier_size.max(commit_worklist.len() + 1);
            max_live_seen_sets = max_live_seen_sets.max(num_live_seen_sets);

            // Update `seen` with the tree and blob IDs reachable from this commit
            let commit_md = self.get_commit_metadata(commit_idx);
            if let Some(tree_idx) = commit_md.tree_idx {
                assert!(tree_worklist.is_empty());
                if seen.insert_tree(tree_idx)? {
                    tree_worklist.push((
                        SmallVec::new(),
                        repo_index.get_tree_oid(tree_idx).unwrap().to_owned(),
                    ));

                    visit_tree(
                        repo,
                        &mut symbols,
                        repo_index,
                        &mut num_trees_introduced,
                        &mut num_blobs_introduced,
                        &mut seen,
                        introduced,
                        &mut tree_buf,
                        &mut tree_worklist,
                        &mut blobs_encountered,
                    )?;
                }
            } else {
                warn!(
                    "Failed to find commit metadata for {}; blob metadata may be incomplete or wrong",
                    commit_md.oid
                );
                // NOTE: if we reach this point, we still need to process the child commits, even
                // though we can't traverse this commit's tree.
                // Otherwise, we spuriously fail later, incorrectly reporting a cycle detected.
            }

            // Propagate this commit's seen set into each of its immediate child commit's seen set.
            // Handle the last child commit specially: it inherits this commit node's seen set,
            // as it will no longer be needed. This optimization reduces memory traffic, especially
            // in the common case of a single commit parent.
            let mut edges = cg.edges_directed(commit_idx, Outgoing).peekable();
            while let Some(edge) = edges.next() {
                let edge_index = edge.id().index();
                if visited_commit_edges.put(edge_index) {
                    error!("Edge {edge_index} already visited -- supposed to be impossible!");
                    continue;
                }

                let child_idx = edge.target();
                let child_seen = &mut seen_sets[child_idx.index()];
                if let Some(child_seen) = child_seen.as_mut() {
                    // Already have a seen set allocated for this child commit. Update it.
                    child_seen.union_update(&seen);
                } else {
                    // No seen set allocated yet for this child commit.
                    // Make one, recycling the current parent commit's seen set if possible.
                    num_live_seen_sets += 1;
                    if edges.peek().is_none() {
                        *child_seen = Some(std::mem::take(&mut seen));
                    } else {
                        *child_seen = Some(seen.clone());
                    }
                }

                // If the child commit node has no unvisited parent commits, add it to the worklist
                if !cg
                    .edges_directed(child_idx, Incoming)
                    .any(|edge| !visited_commit_edges.contains(edge.id().index()))
                {
                    commit_worklist.push((commit_out_degree(child_idx)?, child_idx));
                }
            }
        }

        if visited_commit_edges.count_ones(..) != visited_commit_edges.len() {
            bail!("Topological traversal of commits failed: a commit cycle!?");
        }

        assert_eq!(num_commits_visited, num_commits);
        assert_eq!(visited_commits.len(), num_commits);

        debug!(
            "{num_commits_visited} commits visited; \
              {max_frontier_size} max entries in frontier; \
              {max_live_seen_sets} max live seen sets; \
              {num_trees_introduced} trees introduced; \
              {num_blobs_introduced} blobs introduced; \
              {:.6}s",
            t1.elapsed().as_secs_f64()
        );

        // Massage intermediate accumulated results into output format
        let commit_metadata: Vec<CommitBlobMetadata> = cg
            .node_weights()
            .zip(blobs_introduced)
            .map(|(md, introduced_blobs)| CommitBlobMetadata {
                commit_oid: md.oid,
                introduced_blobs,
            })
            .collect();

        Ok(commit_metadata)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_tree(
    repo: &gix::Repository,
    symbols: &mut BStringTable,
    repo_index: &RepositoryIndex,
    num_trees_introduced: &mut usize,
    num_blobs_introduced: &mut usize,
    seen: &mut SeenObjectSet,
    introduced: &mut IntroducedBlobs,
    tree_buf: &mut Vec<u8>,
    tree_worklist: &mut TreeWorklist,
    blobs_encountered: &mut Vec<ObjectIdx>,
) -> Result<()> {
    blobs_encountered.clear();
    while let Some((name_path, tree_oid)) = tree_worklist.pop() {
        // read the tree object from the repo,
        // enumerate its child entries, and extend the worklist with the unseen child trees
        let tree_iter = unwrap_ok_or_continue!(
            repo.objects.find_tree_iter(&tree_oid, tree_buf),
            |e| error!("Failed to find tree {tree_oid}: {e}"),
        );

        *num_trees_introduced += 1;

        for child in tree_iter {
            let child = unwrap_ok_or_continue!(child, |e| {
                error!("Failed to read tree entry from {tree_oid}: {e}")
            });
            // skip non-tree / non-blob tree entries
            match child.mode.kind() {
                EntryKind::Link | EntryKind::Commit => continue,

                EntryKind::Tree => {
                    let child_idx =
                        unwrap_some_or_continue!(repo_index.get_tree_index(child.oid), || error!(
                            "Failed to find tree index for {} from tree {tree_oid}",
                            child.oid
                        ),);
                    if !seen.insert_tree(child_idx)? {
                        continue;
                    }
                    let mut child_name_path = name_path.clone();
                    child_name_path.push(symbols.get_or_intern(child.filename));
                    tree_worklist.push((child_name_path, child.oid.to_owned()));
                }

                EntryKind::Blob | EntryKind::BlobExecutable => {
                    let child_idx =
                        unwrap_some_or_continue!(repo_index.get_blob_index(child.oid), || error!(
                            "Failed to find blob index for {} from tree {tree_oid}",
                            child.oid
                        ));
                    if seen.contains_blob(child_idx)? {
                        continue;
                    }
                    blobs_encountered.push(child_idx);

                    *num_blobs_introduced += 1;

                    // Compute full path to blob as a bytestring.
                    // Instead of using `bstr::join`, manually construct the string to
                    // avoid intermediate allocations.
                    let name_path = {
                        use bstr::ByteVec;

                        let fname = symbols.get_or_intern(child.filename);

                        let needed_len = name_path.iter().map(|s| s.len()).sum::<usize>()
                            + child.filename.len()
                            + name_path.len();
                        let mut it = name_path
                            .iter()
                            .copied()
                            .chain(std::iter::once(fname))
                            .map(|s| symbols.resolve(s));
                        let mut buf = Vec::with_capacity(needed_len);
                        if let Some(p) = it.next() {
                            buf.push_str(p);
                            for p in it {
                                buf.push_char('/');
                                buf.push_str(p);
                            }
                        }
                        debug_assert_eq!(needed_len, buf.capacity());
                        debug_assert_eq!(needed_len, buf.len());
                        BString::from(buf)
                    };
                    introduced.push((child.oid.to_owned(), name_path));
                }
            }
        }
    }

    for blob_idx in blobs_encountered.iter() {
        seen.insert_blob(*blob_idx)?;
    }
    blobs_encountered.clear();

    Ok(())
}
