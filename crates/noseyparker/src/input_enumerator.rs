#![allow(dead_code)]

use anyhow::{Context, Result};
use ignore::{DirEntry, WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, debug_span, error, error_span, info, warn};

use anyhow::bail;
use bstr::{BStr, BString, ByteSlice};
use gix::hashtable::{hash_map, HashMap};
use gix::ObjectId;
use petgraph::graph::{DiGraph, EdgeIndex, IndexType, NodeIndex};

use crate::bstring_table::BStringTable;

use crate::progress::Progress;

pub struct FilesystemEnumeratorResult {
    pub files: Vec<FileResult>,
    pub git_repos: Vec<GitRepoResult>,
}

impl FilesystemEnumeratorResult {
    pub fn total_blob_bytes(&self) -> u64 {
        let git_blob_bytes: u64 = self.git_repos.iter().map(|e| e.total_blob_bytes()).sum();
        let file_bytes: u64 = self.files.iter().map(|e| e.num_bytes).sum();
        git_blob_bytes + file_bytes
    }
}

pub struct FileResult {
    pub path: PathBuf,
    pub num_bytes: u64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct BlobSeenIn {
    pub commit_oid: ObjectId,
    pub path: BString,
}

impl BlobSeenIn {
    pub fn path(&self) -> Result<&Path, bstr::Utf8Error> {
        self.path.to_path()
    }
}

#[derive(Clone)]
pub struct BlobMetadata {
    pub blob_oid: ObjectId,
    pub num_bytes: u64,
    pub first_seen: SmallVec<[BlobSeenIn; 1]>,
}

pub struct GitRepoResult {
    pub path: PathBuf,
    pub blobs: Vec<BlobMetadata>,
}

impl GitRepoResult {
    pub fn total_blob_bytes(&self) -> u64 {
        self.blobs.iter().map(|t| t.num_bytes).sum()
    }

    pub fn num_blobs(&self) -> u64 {
        self.blobs.len() as u64
    }
}

struct VisitorBuilder<'t> {
    max_file_size: Option<u64>,

    global_files: &'t Mutex<Vec<FileResult>>,
    global_git_repos: &'t Mutex<Vec<GitRepoResult>>,

    progress: Progress,
}

impl<'s, 't> ignore::ParallelVisitorBuilder<'s> for VisitorBuilder<'t>
where
    't: 's,
{
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(Visitor {
            max_file_size: self.max_file_size,
            local_files: Vec::new(),
            local_git_repos: Vec::new(),
            global_files: self.global_files,
            global_git_repos: self.global_git_repos,

            progress: self.progress.clone(),
        })
    }
}

struct Visitor<'t> {
    max_file_size: Option<u64>,

    local_files: Vec<FileResult>,
    local_git_repos: Vec<GitRepoResult>,

    global_files: &'t Mutex<Vec<FileResult>>,
    global_git_repos: &'t Mutex<Vec<GitRepoResult>>,

    progress: Progress,
}

impl<'t> Visitor<'t> {
    #[inline]
    fn file_too_big(&self, size: u64) -> bool {
        self.max_file_size.map_or(false, |max_size| size > max_size)
    }
}

impl<'t> Drop for Visitor<'t> {
    fn drop(&mut self) {
        self.global_files
            .lock()
            .unwrap()
            .extend(std::mem::take(&mut self.local_files));
        self.global_git_repos
            .lock()
            .unwrap()
            .extend(std::mem::take(&mut self.local_git_repos));
    }
}

impl<'t> ignore::ParallelVisitor for Visitor<'t> {
    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
        // FIXME: dedupe based on (device, inode) on platforms where available; see https://docs.rs/same-file/1.0.6/same_file/ for ideas

        let entry = match result {
            Err(e) => {
                error!("Failed to get entry: {}; skipping", e);
                return WalkState::Skip;
            }
            Ok(v) => v,
        };

        let path = entry.path();
        let metadata = match entry.metadata() {
            Err(e) => {
                error!("Failed to get metadata for {}: {}; skipping", path.display(), e);
                return WalkState::Skip;
            }
            Ok(v) => v,
        };

        if metadata.is_file() {
            let bytes = metadata.len();
            let path = path.to_owned();
            if self.file_too_big(bytes) {
                debug!("Skipping {}: size {} exceeds max size", path.display(), bytes);
            } else {
                self.progress.inc(bytes);
                self.local_files.push(FileResult {
                    path,
                    num_bytes: bytes,
                });
            }
        } else if metadata.is_dir() {
            match open_git_repo(path) {
                Err(e) => {
                    error!("Failed to open Git repository at {}: {}; skipping", path.display(), e);
                    return WalkState::Skip;
                }
                Ok(Some(repository)) => {
                    debug!("Found Git repo at {}", path.display());
                    let enumerator = GitRepoEnumerator::new(path, &repository);
                    let blobs = match enumerator.run(&mut self.progress) {
                        Err(e) => {
                            error!(
                                "Failed to enumerate Git repository at {:?}: {}; skipping",
                                path, e
                            );
                            return WalkState::Skip;
                        }
                        Ok(v) => v.blobs,
                    };
                    let path = path.to_owned();
                    self.local_git_repos.push(GitRepoResult { path, blobs })
                }
                Ok(None) => {}
            }
        } else if metadata.is_symlink() {
            // No problem; just ignore it
            //
            // Had follow_symlinks been enabled, the pointed-to entry would have been yielded
            // instead.
        } else {
            warn!("Unhandled path type: {}: {:?}", path.display(), entry.file_type());
        }
        WalkState::Continue
    }
}

/// Provides capabitilies to recursively enumerate a filesystem.
///
/// This provides a handful of features, including:
///
/// - Enumeration of found files
/// - Enumeration of blobs found in Git repositories
/// - Support for ignoring files based on size or using path-based gitignore-style rules
pub struct FilesystemEnumerator {
    walk_builder: WalkBuilder,

    // We store the max file size here in addition to inside the `walk_builder` to work around a
    // bug in `ignore` where max filesize is not applied to top-level file inputs, only inputs that
    // appear under a directory.
    max_file_size: Option<u64>,
}

impl FilesystemEnumerator {
    pub const DEFAULT_MAX_FILESIZE: u64 = 100 * 1024 * 1024;
    pub const DEFAULT_FOLLOW_LINKS: bool = false;

    /// Create a new `FilesystemEnumerator` with the given set of input roots using default
    /// settings.
    ///
    /// The default maximum file size is 100 MiB.
    ///
    /// The default behavior is to not follow symlinks.
    pub fn new<T: AsRef<Path>>(inputs: &[T]) -> Result<Self> {
        let mut builder = WalkBuilder::new(&inputs[0]);
        for input in &inputs[1..] {
            builder.add(input);
        }
        let max_file_size = Some(Self::DEFAULT_MAX_FILESIZE);
        builder.follow_links(Self::DEFAULT_FOLLOW_LINKS);
        builder.max_filesize(max_file_size);
        builder.standard_filters(false);

        Ok(FilesystemEnumerator {
            walk_builder: builder,
            max_file_size,
        })
    }

    /// Set the number of parallel enumeration threads.
    pub fn threads(&mut self, threads: usize) -> &mut Self {
        self.walk_builder.threads(threads);
        self
    }

    /// Add a set of gitignore-style rules from the given ignore file.
    pub fn add_ignore<T: AsRef<Path>>(&mut self, path: T) -> Result<&mut Self> {
        match self.walk_builder.add_ignore(path) {
            Some(e) => Err(e)?,
            None => Ok(self),
        }
    }

    /// Enable or disable whether symbolic links are followed.
    pub fn follow_links(&mut self, follow_links: bool) -> &mut Self {
        self.walk_builder.follow_links(follow_links);
        self
    }

    /// Set the maximum file size for enumerated files.
    ///
    /// Files larger than this value will be skipped.
    pub fn max_filesize(&mut self, max_filesize: Option<u64>) -> &mut Self {
        self.walk_builder.max_filesize(max_filesize);
        self.max_file_size = max_filesize;
        self
    }

    /// Specify an ad-hoc filtering function to control which entries are enumerated.
    ///
    /// This can be used to skip entire directories.
    pub fn filter_entry<P>(&mut self, filter: P) -> &mut Self
    where
        P: Fn(&DirEntry) -> bool + Send + Sync + 'static,
    {
        self.walk_builder.filter_entry(filter);
        self
    }

    pub fn run(&self, progress: &Progress) -> Result<FilesystemEnumeratorResult> {
        let files = Mutex::new(Vec::new());
        let git_repos = Mutex::new(Vec::new());

        let mut visitor_builder = VisitorBuilder {
            max_file_size: self.max_file_size,
            global_files: &files,
            global_git_repos: &git_repos,
            progress: progress.clone(),
        };

        self.walk_builder
            .build_parallel()
            .visit(&mut visitor_builder);

        let files = files.into_inner()?;
        let git_repos = git_repos.into_inner().unwrap();

        Ok(FilesystemEnumeratorResult { files, git_repos })
    }
}

/// Opens the given Git repository if it exists, returning None otherwise.
pub fn open_git_repo(path: &Path) -> Result<Option<gix::Repository>> {
    let opts = gix::open::Options::isolated().open_path_as_is(true);
    match gix::open_opts(path, opts) {
        Err(gix::open::Error::NotARepository { .. }) => Ok(None),
        Err(err) => Err(err.into()),
        Ok(repo) => Ok(Some(repo)),
    }
}

pub struct GitRepoEnumerator<'a> {
    path: &'a Path,
    repo: &'a gix::Repository,
}

impl<'a> GitRepoEnumerator<'a> {
    pub fn new(path: &'a Path, repo: &'a gix::Repository) -> Self {
        Self { path, repo }
    }

    pub fn run(&self, progress: &mut Progress) -> Result<GitRepoResult> {
        use gix::object::Kind;
        use gix::odb::store::iter::Ordering;
        use gix::prelude::*;

        let _span = error_span!("git_enumerator", "{}", self.path.display()).entered();

        macro_rules! warn {
            ($($arg:expr),*) => {
                progress.suspend(|| {
                    tracing::warn!($($arg),*);
                })
            }
        }

        macro_rules! info {
            ($($arg:expr),*) => {
                progress.suspend(|| {
                    tracing::info!($($arg),*);
                })
            }
        }

        macro_rules! unwrap_or_continue {
            ($arg:expr) => {
                match $arg {
                    Ok(v) => v,
                    Err(e) => {
                        continue;
                    }
                }
            };
            ($arg:expr, $on_error:expr) => {
                match $arg {
                    Ok(v) => v,
                    Err(e) => {
                        $on_error(e);
                        continue;
                    }
                }
            };
        }

        let odb = &self.repo.objects;

        // info!("Counting objects...");

        // TODO: measure how helpful or pointless it is to count the objects in advance
        // FIXME: if keeping the pre-counting step, add some new kind of progress indicator

        // First count the objects to figure out how big to allocate data structures
        // We are assuming that the repository doesn't change in the meantime.
        // If it does, our allocation estimates won't be right. Too bad!
        let t1 = Instant::now();
        let (num_commits, num_trees, num_blobs) = {
            let mut num_commits = 0;
            let mut num_trees = 0;
            let mut num_blobs = 0;

            for oid in odb
                .iter()
                .context("Failed to iterate object database")?
                .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
            {
                let oid = unwrap_or_continue!(oid, |e| warn!("Failed to read object id: {e}"));
                let hdr = unwrap_or_continue!(odb.header(oid), |e| warn!(
                    "Failed to read object header for {oid}: {e}"
                ));
                match hdr.kind() {
                    Kind::Commit => num_commits += 1,
                    Kind::Tree => num_trees += 1,
                    Kind::Blob => num_blobs += 1,
                    _ => {}
                }
            }
            (num_commits, num_trees, num_blobs)
        };
        // info!(
        //     "Counted {num_commits} commits, {num_trees} trees, and {num_blobs} blobs in {:.6}s",
        //     t1.elapsed().as_secs_f64()
        // );

        let mut blobs: Vec<(ObjectId, u64)> = Vec::with_capacity(num_blobs);
        let mut metadata_graph = GitMetadataGraph::with_capacity(num_commits, num_trees, num_blobs);

        // scratch buffer used for decoding commits and trees.
        // size chosen here based on experimentation: biggest commit/tree in cpython is 250k
        let orig_scratch_capacity = 1024 * 1024;
        let mut scratch: Vec<u8> = Vec::with_capacity(orig_scratch_capacity);

        for oid in odb
            .iter()
            .context("Failed to iterate object database")?
            .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
        {
            let oid = unwrap_or_continue!(oid, |e| warn!("Failed to read object id: {e}"));
            let hdr = unwrap_or_continue!(odb.header(oid), |e| warn!(
                "Failed to read object header for {oid}: {e}"
            ));
            match hdr.kind() {
                Kind::Blob => {
                    let obj_size = hdr.size();
                    metadata_graph.get_blob_idx(oid);
                    blobs.push((oid, obj_size));
                    progress.inc(obj_size);
                }

                Kind::Commit => {
                    let commit = unwrap_or_continue!(odb.find_commit(oid, &mut scratch), |e| {
                        // NOTE: resolution of this will improve things: https://github.com/Byron/gitoxide/issues/950
                        warn!("Failed to find commit {oid}: {e}");

                        // let mut scratch2 = Vec::new();
                        // let res = odb.try_find(oid, &mut scratch2);
                        // warn!("try_find result: {res:?}");
                        // match res {
                        //     Err(e) => warn!("error finding object {oid}: {e:?}"),
                        //     Ok(None) => warn!("error finding object {oid}: object not found??"),
                        //     Ok(Some(d)) => {
                        //         match d.decode() {
                        //             Err(e) => warn!("error decoding object {oid}: {e:?}"),
                        //             Ok(o) => {
                        //                 match o.into_commit() {
                        //                     None => warn!("failed to convert data from {oid} into commit"),
                        //                     Some(c) => warn!("got commit! {c:?}"),
                        //                 }
                        //             }
                        //         }
                        //     }
                        // }
                    });

                    let tree_idx = metadata_graph.get_tree_idx(commit.tree());
                    let commit_idx = metadata_graph.get_commit_idx(oid, Some(tree_idx));
                    for parent_oid in commit.parents() {
                        let parent_idx = metadata_graph.get_commit_idx(parent_oid, None);
                        metadata_graph.add_commit_edge(parent_idx, commit_idx);
                    }
                }

                Kind::Tree => {
                    let tree_idx = metadata_graph.get_tree_idx(oid);
                    let tree_ref = unwrap_or_continue!(odb.find_tree(oid, &mut scratch), |e| {
                        warn!("Failed to find tree {oid}: {e}");
                    });
                    for child in tree_ref.entries {
                        use gix::objs::tree::EntryMode;
                        let child_idx = match child.mode {
                            EntryMode::Tree => metadata_graph.get_tree_idx(child.oid.into()),
                            EntryMode::Blob | EntryMode::BlobExecutable => {
                                metadata_graph.get_blob_idx(child.oid.into())
                            }
                            _ => continue,
                        };

                        metadata_graph.add_tree_edge(
                            tree_idx,
                            child_idx,
                            child.filename.to_owned(),
                        );
                    }
                }

                _ => {}
            }
        }

        // Some performance sanity checks
        {
            if scratch.capacity() != orig_scratch_capacity {
                warn!("scratch had to be resized to {}", scratch.capacity());
            }

            if blobs.capacity() != num_blobs {
                warn!("blobs had to be resized to {}", blobs.capacity());
            }

            let commit_nodes_capacity = metadata_graph.commits.capacity().0;
            if commit_nodes_capacity != num_commits {
                warn!("commits graph nodes had to be resized to {}", commit_nodes_capacity);
            }

            let tree_and_blob_nodes_capacity = metadata_graph.trees_and_blobs.capacity().0;
            if tree_and_blob_nodes_capacity != num_trees + num_blobs {
                warn!(
                    "tree/blob graph nodes had to be resized to {}",
                    tree_and_blob_nodes_capacity
                );
            }
        }

        let path = self.path.to_owned();
        match repo_metadata(&metadata_graph, &progress) {
            Err(e) => {
                warn!("failed to compute reachable blobs: {e}");
                let blobs = blobs
                    .into_iter()
                    .map(|(blob_oid, num_bytes)| BlobMetadata {
                        blob_oid,
                        num_bytes,
                        first_seen: Default::default(),
                    })
                    .collect();
                Ok(GitRepoResult { path, blobs })
            }
            Ok(md) => {
                // FIXME: apply path-based ignore rules to blobs here, like the filesystem enumerator
                let mut inverted = HashMap::<ObjectId, SmallVec<[BlobSeenIn; 1]>>::with_capacity_and_hasher(num_blobs, Default::default());
                for e in md.into_iter() {
                    for (blob_oid, path) in e.introduced_blobs.into_iter() {
                        let vals = inverted.entry(blob_oid).or_insert(SmallVec::new());
                        vals.push(BlobSeenIn{commit_oid: e.commit_oid, path });
                    }
                }

                let blobs = blobs
                    .into_iter()
                    .map(|(blob_oid, num_bytes)| {
                        let first_seen = inverted.get(&blob_oid).map_or(SmallVec::new(), |v| v.clone());
                        BlobMetadata { blob_oid, num_bytes, first_seen }
                    })
                    .collect();
                Ok(GitRepoResult { path, blobs })
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Git Metadata Collection
// -------------------------------------------------------------------------------------------------

type Symbol = crate::bstring_table::Symbol<u32>;

/// A newtype for commit graph indexes, to prevent mixing up indexes from different types of graphs
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Default, Debug)]
struct CommitGraphIdx(NodeIndex);

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
struct TreeBlobGraphIdx(NodeIndex);

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

type TreeBlobNodeIdx = NodeIndex<TreeBlobGraphIdx>;
type TreeBlobEdgeIdx = EdgeIndex<TreeBlobGraphIdx>;

struct CommitMetadata {
    pub oid: ObjectId,
    pub tree_idx: Option<TreeBlobNodeIdx>,
}

#[derive(PartialEq, Eq, Debug)]
enum TreeBlobKind {
    Tree,
    Blob,
}

struct TreeBlobMetadata {
    kind: TreeBlobKind,
    oid: ObjectId,
}

/// A graph of metadata in a Git repository
struct GitMetadataGraph {
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
    // #[inline]
    pub fn get_commit_idx(
        &mut self,
        oid: ObjectId,
        tree_idx: Option<TreeBlobNodeIdx>,
    ) -> CommitNodeIdx {
        match self.commit_oid_to_node_idx.entry(oid) {
            hash_map::Entry::Occupied(e) => {
                let idx = *e.get();
                if tree_idx.is_some() {
                    let mut md = self
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
    // #[inline]
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
    // #[inline]
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

struct RepoMetadata {
    /// index of the commit this is for; indexes into the commits graph
    commit_oid: ObjectId,

    /// set of introduced blobs and path names
    introduced_blobs: Vec<(ObjectId, BString)>,
}

use fixedbitset::FixedBitSet;
use petgraph::prelude::*;
use petgraph::visit::Visitable;
use roaring::RoaringBitmap;
use smallvec::SmallVec;
use std::collections::BinaryHeap;

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

fn repo_metadata(graph: &GitMetadataGraph, progress: &Progress) -> Result<Vec<RepoMetadata>> {
    let t1 = Instant::now();
    let symbols = &graph.symbols;
    let cg = &graph.commits;
    let tbg = &graph.trees_and_blobs;
    let num_commits = cg.node_count();

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

    // FIXME: use a better representation here that is flatter and smaller and allocates less
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

        let mut seen =
            std::mem::replace(&mut seen_sets[commit_index], None).expect("should have a seen set");
        assert!(num_live_seen_sets > 0);
        num_live_seen_sets -= 1;

        num_commits_visited += 1;
        max_frontier_size = max_frontier_size.max(worklist.len() + 1);
        max_live_seen_sets = max_live_seen_sets.max(num_live_seen_sets);

        // Update `seen` with the tree and blob IDs reachable from this commit
        let commit_md = graph.get_commit_metadata(commit_idx);
        // FIXME: improve this type to avoid a runtime check here
        match commit_md.tree_idx {
            None => {
                warn!(
                    "commit metadata missing for {}; blob metadata may be incomplete or wrong",
                    commit_md.oid
                );
                // NOTE: if we reach this point, we still need to enumerate child nodes; we simply
                // can't traverse the commit tree. Otherwise, we spuriously fail later on,
                // reporting a cycle detected
            }
            Some(tree_idx) => {
                assert!(tree_worklist.is_empty());
                // tree_worklist.clear();
                if !seen.contains(tree_idx)? {
                    tree_worklist.push((tree_idx, SmallVec::new()));
                }

                while let Some((idx, name_path)) = tree_worklist.pop() {
                    let metadata = graph.get_tree_blob_metadata(idx);
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

        // FIXME: bequeath `seen` to the last child to avoid a copy, especially in the common case of 1 commit parent
        for edge in cg.edges_directed(commit_idx, Outgoing) {
            let edge_index = edge.id().index();
            if visited_edges.put(edge_index) {
                continue;
            }

            let child_idx = edge.target();

            let child_seen = &mut seen_sets[child_idx.index()];
            match child_seen.as_mut() {
                Some(child_seen) => child_seen.union_update(&seen),
                None => {
                    num_live_seen_sets += 1;
                    *child_seen = Some(seen.clone());
                }
            }

            // If the child node has no unvisited parents, add it to the worklist
            if let None = cg
                .edges_directed(child_idx, Incoming)
                .filter(|edge| !visited_edges.contains(edge.id().index()))
                .next()
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
        info!("{num_commits_visited} commits visited; \
              {max_frontier_size} max entries in frontier; \
              {max_live_seen_sets} max live seen sets; \
              {num_trees_introduced} trees introduced; \
              {num_blobs_introduced} blobs introduced; \
              {:.6}s", t1.elapsed().as_secs_f64());
    });

    // Massage intermediate accumulated results into output format
    let commit_metadata = cg
        .node_weights()
        .zip(blobs_introduced.into_iter())
        .map(|(md, intro)| RepoMetadata {
            commit_oid: md.oid,
            introduced_blobs: intro,
        })
        .collect();
    Ok(commit_metadata)
}
