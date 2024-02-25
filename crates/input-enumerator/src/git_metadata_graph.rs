use anyhow::{bail, Context, Result};
use bstr::BString;
use fixedbitset::FixedBitSet;
use gix::hashtable::{hash_map, HashMap};
use gix::ObjectId;
use petgraph::graph::{DiGraph, EdgeIndex, IndexType, NodeIndex};
use petgraph::prelude::*;
use petgraph::visit::Visitable;
use roaring::RoaringBitmap;
use smallvec::SmallVec;
use std::collections::BinaryHeap;
use std::time::Instant;
use tracing::{debug, error, warn};

use crate::bstring_table::BStringTable;
use progress::Progress;

type Symbol = crate::bstring_table::Symbol<u32>;

/// A newtype for commit graph indexes, to prevent mixing up indexes from different types of graphs
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Default, Debug)]
pub struct CommitGraphIdx(NodeIndex);

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

/// A newtype for tree and blob graph indexes, to prevent mixing up indexes from different types of graphs
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Default, Debug)]
pub struct TreeBlobGraphIdx(NodeIndex);

/// Boilerplate instance for the index newtype
unsafe impl IndexType for TreeBlobGraphIdx {
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

pub type TreeBlobNodeIdx = NodeIndex<TreeBlobGraphIdx>;
pub type TreeBlobEdgeIdx = EdgeIndex<TreeBlobGraphIdx>;

pub struct CommitMetadata {
    pub oid: ObjectId,
    pub tree_idx: Option<TreeBlobNodeIdx>,
}

#[derive(PartialEq, Eq, Debug)]
enum TreeBlobKind {
    Tree,
    Blob,
}

pub struct TreeBlobMetadata {
    kind: TreeBlobKind,
    oid: ObjectId,
}

#[derive(Clone, Debug)]
struct SeenTreeBlobSet(RoaringBitmap);

impl SeenTreeBlobSet {
    #[inline]
    pub fn new() -> Self {
        SeenTreeBlobSet(RoaringBitmap::new())
    }

    #[inline]
    pub fn contains(&self, idx: TreeBlobNodeIdx) -> Result<bool> {
        let idx = idx
            .index()
            .try_into()
            .context("index should be representable with a u32")?;
        Ok(self.0.contains(idx))
    }

    #[inline]
    pub fn insert(&mut self, idx: TreeBlobNodeIdx) -> Result<bool> {
        let idx = idx
            .index()
            .try_into()
            .context("index should be representable with a u32")?;
        Ok(self.0.insert(idx))
    }

    #[inline]
    pub fn union_update(&mut self, other: &Self) {
        self.0 |= &other.0;
    }
}

impl Default for SeenTreeBlobSet {
    #[inline]
    fn default() -> Self {
        Self::new()
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
pub struct GitMetadataGraph {
    symbols: BStringTable,

    commit_oid_to_node_idx: HashMap<ObjectId, CommitNodeIdx>,
    commits: DiGraph<CommitMetadata, (), CommitGraphIdx>,

    tree_blob_oid_to_node_idx: HashMap<ObjectId, TreeBlobNodeIdx>,
    trees_and_blobs: DiGraph<TreeBlobMetadata, Symbol, TreeBlobGraphIdx>,
}

impl GitMetadataGraph {
    /// Create a new commit graph with the given capacity.
    pub fn with_capacity(num_commits: usize, num_trees: usize, num_blobs: usize) -> Self {
        let num_trees_and_blobs = num_trees + num_blobs;

        // use 2x the number of commits, assuming that most commits have a single parent commit,
        // except merges, which usually have 2
        let commit_edges_capacity = num_commits * 2;

        let tree_blob_edges_capacity = {
            // this is an estimate of how many trees+blobs change in a typical commit
            let avg_increase = (num_trees_and_blobs as f64 / num_commits as f64).max(1.0);

            // guess at how many trees+blobs do we expect to see total in a typical commit
            // assume that 6% of a tree's contents change at each commit
            //
            // XXX: this is magic, but chosen empirically from looking at a few test repos.
            //      it would be better if we could sample some commits
            let baseline = avg_increase / 0.06;

            (num_commits as f64 * (baseline + avg_increase)) as usize
        }
        .min(1024 * 1024 * 1024);
        // info!("tree blob edges capacity: {tree_blob_edges_capacity}");

        Self {
            symbols: BStringTable::new(),

            commit_oid_to_node_idx: HashMap::with_capacity_and_hasher(
                num_commits,
                Default::default(),
            ),
            commits: DiGraph::with_capacity(num_commits, commit_edges_capacity),
            trees_and_blobs: DiGraph::with_capacity(num_trees_and_blobs, tree_blob_edges_capacity),
            tree_blob_oid_to_node_idx: HashMap::with_capacity_and_hasher(
                num_trees_and_blobs,
                Default::default(),
            ),
        }
    }

    /// Get the commit metadata for the given graph node index.
    ///
    /// Panics if the given graph node index is not valid for this graph.
    #[inline]
    pub fn get_commit_metadata(&self, idx: CommitNodeIdx) -> &CommitMetadata {
        self.commits
            .node_weight(idx)
            .expect("commit graph node index should be valid")
    }

    /// Get the tree/blob metadata for the given graph node index.
    ///
    /// Panics if the given graph node index is not valid for this graph.
    #[inline]
    pub fn get_tree_blob_metadata(&self, idx: TreeBlobNodeIdx) -> &TreeBlobMetadata {
        self.trees_and_blobs
            .node_weight(idx)
            .expect("tree/blob graph node index should be valid")
    }

    /// Get the index of the graph node for the given commit, creating it if needed.
    ///
    /// If a node already exists for the given commit and `tree_idx` is given, the node's metadata
    /// is updated with the given value.
    pub fn get_commit_idx(
        &mut self,
        oid: ObjectId,
        tree_idx: Option<TreeBlobNodeIdx>,
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
    pub fn add_commit_edge(
        &mut self,
        parent_idx: CommitNodeIdx,
        child_idx: CommitNodeIdx,
    ) -> CommitEdgeIdx {
        // For alternative behavior that doesn't add parallel edges, use
        // `self.commits.update_edge(parent_idx, child_idx, ())`.
        self.commits.add_edge(parent_idx, child_idx, ())
    }

    /// Add a new graph node for the given blob object, returning its index.
    pub fn get_blob_idx(&mut self, blob_oid: ObjectId) -> TreeBlobNodeIdx {
        *self
            .tree_blob_oid_to_node_idx
            .entry(blob_oid)
            .or_insert_with(|| {
                self.trees_and_blobs.add_node(TreeBlobMetadata {
                    kind: TreeBlobKind::Blob,
                    oid: blob_oid,
                })
            })
    }

    /// Add a new graph node for the given tree object, returning its index.
    pub fn get_tree_idx(&mut self, tree_oid: ObjectId) -> TreeBlobNodeIdx {
        *self
            .tree_blob_oid_to_node_idx
            .entry(tree_oid)
            .or_insert_with(|| {
                self.trees_and_blobs.add_node(TreeBlobMetadata {
                    kind: TreeBlobKind::Tree,
                    oid: tree_oid,
                })
            })
    }

    /// Add a new edge for a tree and another tree or blob.
    ///
    /// NOTE: If such an edge already exists, a parallel edge is added.
    pub fn add_tree_edge(
        &mut self,
        parent_idx: TreeBlobNodeIdx,
        child_idx: TreeBlobNodeIdx,
        name: BString,
    ) -> TreeBlobEdgeIdx {
        // NOTE: this will allow parallel (i.e., duplicate) edges from `from_idx` to `to_idx`.
        //
        // For alternative behavior that doesn't add parallel edges, use
        // `self.commits.update_edge(from_idx, to_idx, ())`.
        let sym = self.symbols.get_or_intern(name);
        self.trees_and_blobs.add_edge(parent_idx, child_idx, sym)
    }

    pub fn num_commits(&self) -> usize {
        self.commits.node_count()
    }

    pub fn num_trees(&self) -> usize {
        self.trees_and_blobs
            .node_weights()
            .filter(|md| md.kind == TreeBlobKind::Tree)
            .count()
    }

    pub fn num_blobs(&self) -> usize {
        self.trees_and_blobs
            .node_weights()
            .filter(|md| md.kind == TreeBlobKind::Blob)
            .count()
    }
}

pub struct RepoMetadata {
    /// index of the commit this is for; indexes into the commits graph
    pub commit_oid: ObjectId,

    /// set of introduced blobs and path names
    pub introduced_blobs: Vec<(ObjectId, BString)>,
}

impl GitMetadataGraph {
    pub fn repo_metadata(&self, progress: &Progress) -> Result<Vec<RepoMetadata>> {
        let t1 = Instant::now();
        let symbols = &self.symbols;
        let cg = &self.commits;
        let tbg = &self.trees_and_blobs;
        let num_commits = cg.node_count();

        // The set of seen trees and blobs. This has an entry for _each_ commit, though at runtime,
        // not all of these seen sets will be "live".
        //
        // FIXME: merge this data structure with the `worklist` priority queue; See https://docs.rs/priority-queue; this allows O(1) updates of items in the queue
        let mut seen_sets: Vec<Option<SeenTreeBlobSet>> = vec![None; num_commits];

        let mut blobs_introduced: Vec<Vec<(ObjectId, BString)>> = vec![Vec::new(); num_commits];

        // An adapatation of Kahn's topological sorting algorithm, to visit the commit nodes in
        // topological order: <https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm>
        // This algorithm naturally mantains a frontier of still-to-expand nodes.
        //
        // We attach to each node in the frontier a set of seen blobs and seen trees in the traversal
        // up to that point.

        // NOTE: petgraph comes with a pre-built data type for keeping track of visited nodes,
        // but has no such thing for keeping track of visited edges, so we make our own.
        let mut visited_edges = FixedBitSet::with_capacity(cg.edge_count());

        // Keep track of which commit nodes we have seen: needed to avoid re-visiting nodes in the rare
        // present of parallel (i.e., multiple) edges between two commits.
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

        // A queue of commit graph node indexes, ordered by minimum out-degree
        let mut worklist =
            BinaryHeap::<(OutDegree, CommitNodeIdx)>::with_capacity(1024.max(num_commits / 2));

        // Initialize with commit nodes that have no parents
        for root_idx in cg
            .node_indices()
            .filter(|idx| cg.neighbors_directed(*idx, Incoming).count() == 0)
        {
            let out_degree = commit_out_degree(root_idx)?;
            worklist.push((out_degree, root_idx));
            seen_sets[root_idx.index()] = Some(SeenTreeBlobSet::new());
        }

        let mut tree_worklist: Vec<(TreeBlobNodeIdx, SmallVec<[Symbol; 2]>)> =
            Vec::with_capacity(32 * 1024);

        // various counters for statistics
        let mut max_frontier_size = 0; // max value of size of `worklist`
        let mut num_blobs_introduced = 0; // total number of blobs introduced in commits
        let mut num_trees_introduced = 0; // total number of trees introduced in commits
        let mut num_commits_visited = 0; // total number of commits visited

        let mut num_live_seen_sets = worklist.len();
        let mut max_live_seen_sets = 0; // max value of `num_live_seen_sets`

        while let Some((_out_degree, commit_idx)) = worklist.pop() {
            let commit_index = commit_idx.index();
            if visited_commits.put(commit_index) {
                warn!("found duplicate commit node {commit_index}");
                continue;
            }

            // info!("{commit_index}: {out_degree:?} {:?}", worklist.iter().map(|e| e.0).max());

            let mut seen = seen_sets[commit_index]
                .take()
                .expect("should have a seen set");
            assert!(num_live_seen_sets > 0);
            num_live_seen_sets -= 1;

            num_commits_visited += 1;
            max_frontier_size = max_frontier_size.max(worklist.len() + 1);
            max_live_seen_sets = max_live_seen_sets.max(num_live_seen_sets);

            // Update `seen` with the tree and blob IDs reachable from this commit
            let commit_md = self.get_commit_metadata(commit_idx);
            // FIXME: improve this type to avoid a runtime check here
            match commit_md.tree_idx {
                None => {
                    warn!(
                        "commit metadata missing for {}; blob metadata may be incomplete or wrong",
                        commit_md.oid
                    );
                    // NOTE: if we reach this point, we still need to enumerate child nodes, even
                    // though we can't traverse the commit's tree.
                    // Otherwise, we spuriously fail later, incorrectly reporting a cycle detected.
                }
                Some(tree_idx) => {
                    assert!(tree_worklist.is_empty());
                    if !seen.contains(tree_idx)? {
                        tree_worklist.push((tree_idx, SmallVec::new()));
                    }

                    //while let Some((name_path, idx)) = tree_worklist.pop() {
                    while let Some((idx, name_path)) = tree_worklist.pop() {
                        let metadata = self.get_tree_blob_metadata(idx);
                        match metadata.kind {
                            TreeBlobKind::Tree => num_trees_introduced += 1,
                            TreeBlobKind::Blob => {
                                num_blobs_introduced += 1;

                                let name_path: Vec<u8> =
                                    bstr::join("/", name_path.iter().map(|s| symbols.resolve(*s)));
                                blobs_introduced[commit_index]
                                    .push((metadata.oid, BString::from(name_path)));
                                // info!("{}: {}: {name_path:?}", commit_md.oid, metadata.oid);
                            }
                        }

                        for edge in tbg.edges_directed(idx, Outgoing) {
                            let child_idx = edge.target();
                            if !seen.insert(child_idx)? {
                                continue;
                            }

                            let mut child_name_path = name_path.clone();
                            child_name_path.push(*edge.weight());
                            tree_worklist.push((child_idx, child_name_path));
                        }
                    }
                }
            }

            // Propagate this commit's seen set into each of its immediate child commit's seen set.
            let mut edges = cg.edges_directed(commit_idx, Outgoing).peekable();
            while let Some(edge) = edges.next() {
                let edge_index = edge.id().index();
                if visited_edges.put(edge_index) {
                    error!(
                        "Edge {edge_index} already visited -- this was supposed to be impossible!"
                    );
                    continue;
                }
                let child_idx = edge.target();

                let child_seen = &mut seen_sets[child_idx.index()];
                match child_seen.as_mut() {
                    Some(child_seen) => {
                        // Already have a seen set allocated for this child commit. Update it.
                        child_seen.union_update(&seen);
                    }
                    None => {
                        // No seen set allocated yet for this child commit; make a new one,
                        // recycling the current parent commit's seen set if possible. We can do
                        // this on the last loop iteration (i.e., when we are working on the last
                        // child commit), because at that point, the parent's seen set will no
                        // longer be needed. This optimization reduces memory traffic, especially
                        // in the common case of a single commit parent.

                        num_live_seen_sets += 1;
                        if edges.peek().is_none() {
                            *child_seen = Some(std::mem::take(&mut seen));
                        } else {
                            *child_seen = Some(seen.clone());
                        }
                    }
                }

                // If the child commit node has no unvisited parent commits, add it to the worklist
                if !cg
                    .edges_directed(child_idx, Incoming)
                    .any(|edge| !visited_edges.contains(edge.id().index()))
                {
                    worklist.push((commit_out_degree(child_idx)?, child_idx));
                }
            }
        }

        if visited_edges.count_ones(..) != visited_edges.len() {
            bail!("topological traversal of commits failed: a commit cycle!?");
        }

        assert_eq!(num_commits_visited, num_commits);
        assert_eq!(visited_commits.len(), num_commits);

        progress.suspend(|| {
            debug!(
                "{num_commits_visited} commits visited; \
                  {max_frontier_size} max entries in frontier; \
                  {max_live_seen_sets} max live seen sets; \
                  {num_trees_introduced} trees introduced; \
                  {num_blobs_introduced} blobs introduced; \
                  {:.6}s",
                t1.elapsed().as_secs_f64()
            );
        });

        // Massage intermediate accumulated results into output format
        let commit_metadata = cg
            .node_weights()
            .zip(blobs_introduced)
            .map(|(md, intro)| RepoMetadata {
                commit_oid: md.oid,
                introduced_blobs: intro,
            })
            .collect();
        Ok(commit_metadata)
    }
}
