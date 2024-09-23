use anyhow::{Context, Result};
use gix::{hashtable::HashMap, ObjectId, OdbHandle, Repository};
use ignore::gitignore::Gitignore;
use smallvec::SmallVec;
use std::path::{Path, PathBuf};
use std::time::Instant;
// use std::time::Instant;
use tracing::{debug, debug_span, warn};

use crate::blob_appearance::{BlobAppearance, BlobAppearanceSet};
use crate::git_commit_metadata::CommitMetadata;
use crate::git_metadata_graph::GitMetadataGraph;

// -------------------------------------------------------------------------------------------------
// implementation helpers
// -------------------------------------------------------------------------------------------------
macro_rules! unwrap_or_continue {
    ($arg:expr) => {
        match $arg {
            Ok(v) => v,
            Err(_e) => {
                continue;
            }
        }
    };
    ($arg:expr, $on_error:expr) => {
        match $arg {
            Ok(v) => v,
            Err(e) => {
                #[allow(clippy::redundant_closure_call)]
                $on_error(e);
                continue;
            }
        }
    };
}

pub struct ObjectCounts {
    num_commits: usize,
    num_trees: usize,
    num_blobs: usize,
}

// TODO: measure how helpful or pointless it is to count the objects in advance
fn count_git_objects(odb: &OdbHandle) -> Result<ObjectCounts> {
    let t1 = Instant::now();

    use gix::object::Kind;
    use gix::odb::store::iter::Ordering;
    use gix::prelude::*;

    let mut num_commits = 0;
    let mut num_trees = 0;
    let mut num_blobs = 0;
    let mut num_objects = 0;

    for oid in odb
        .iter()
        .context("Failed to iterate object database")?
        .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
    {
        num_objects += 1;
        let oid = unwrap_or_continue!(oid, |e| { warn!("Failed to read object id: {e}") });
        let hdr = unwrap_or_continue!(odb.header(oid), |e| {
            warn!("Failed to read object header for {oid}: {e}")
        });
        match hdr.kind() {
            Kind::Commit => num_commits += 1,
            Kind::Tree => num_trees += 1,
            Kind::Blob => num_blobs += 1,
            Kind::Tag => {}
        }
    }

    debug!("Counted {num_objects} objects in {:.6}s", t1.elapsed().as_secs_f64());

    Ok(ObjectCounts {
        num_commits,
        num_trees,
        num_blobs,
    })
}

// -------------------------------------------------------------------------------------------------
// enumeration return types
// -------------------------------------------------------------------------------------------------
pub struct GitRepoResult {
    /// Path to the repository clone
    pub path: PathBuf,

    /// The opened Git repository
    pub repository: Repository,

    /// The blobs to be scanned
    pub blobs: Vec<BlobMetadata>,

    /// Finite map from commit ID to metadata
    ///
    /// NOTE: this may be incomplete, missing entries for some commits
    pub commit_metadata: HashMap<ObjectId, CommitMetadata>,
}

impl GitRepoResult {
    pub fn total_blob_bytes(&self) -> u64 {
        self.blobs.iter().map(|t| t.num_bytes).sum()
    }

    pub fn num_blobs(&self) -> u64 {
        self.blobs.len() as u64
    }
}

#[derive(Clone)]
pub struct BlobMetadata {
    pub blob_oid: ObjectId,
    pub num_bytes: u64,
    pub first_seen: BlobAppearanceSet,
}

// -------------------------------------------------------------------------------------------------
// git repo enumerator, with metadata
// -------------------------------------------------------------------------------------------------
pub struct GitRepoWithMetadataEnumerator<'a> {
    path: &'a Path,
    repo: Repository,
    gitignore: &'a Gitignore,
}

impl<'a> GitRepoWithMetadataEnumerator<'a> {
    pub fn new(path: &'a Path, repo: Repository, gitignore: &'a Gitignore) -> Self {
        Self {
            path,
            repo,
            gitignore,
        }
    }

    pub fn run(self) -> Result<GitRepoResult> {
        let t1 = Instant::now();

        use gix::object::Kind;
        use gix::odb::store::iter::Ordering;
        use gix::prelude::*;

        let _span = debug_span!("enumerate_git_with_metadata", "{}", self.path.display()).entered();

        let odb = &self.repo.objects;

        // TODO: measure how helpful or pointless it is to count the objects in advance

        // First count the objects to figure out how big to allocate data structures.
        // We're assuming that the repository doesn't change in the meantime.
        // If it does, our allocation estimates won't be right. Too bad!
        let ObjectCounts {
            num_commits,
            num_trees,
            num_blobs,
        } = count_git_objects(odb)?;

        let mut blobs: Vec<(ObjectId, u64)> = Vec::with_capacity(num_blobs);
        let mut metadata_graph = GitMetadataGraph::with_capacity(num_commits, num_trees, num_blobs);

        // scratch buffer used for decoding commits and trees.
        // size chosen here based on experimentation: biggest commit/tree in cpython is 250k
        let orig_scratch_capacity = 1024 * 1024;
        let mut scratch: Vec<u8> = Vec::with_capacity(orig_scratch_capacity);

        let mut commit_metadata =
            HashMap::with_capacity_and_hasher(num_commits, Default::default());

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
                }

                Kind::Commit => {
                    let commit = unwrap_or_continue!(odb.find_commit(&oid, &mut scratch), |e| {
                        warn!("Failed to find commit {oid}: {e}");
                    });

                    let tree_idx = metadata_graph.get_tree_idx(commit.tree());
                    let commit_idx = metadata_graph.get_commit_idx(oid, Some(tree_idx));
                    for parent_oid in commit.parents() {
                        let parent_idx = metadata_graph.get_commit_idx(parent_oid, None);
                        metadata_graph.add_commit_edge(parent_idx, commit_idx);
                    }

                    let committer = &commit.committer;
                    let author = &commit.author;
                    let md = CommitMetadata {
                        commit_id: oid,
                        committer_name: committer.name.to_owned(),
                        committer_timestamp: committer.time,
                        committer_email: committer.email.to_owned(),
                        author_name: author.name.to_owned(),
                        author_timestamp: author.time,
                        author_email: author.email.to_owned(),
                        message: commit.message.to_owned(),
                    };
                    commit_metadata.insert(oid, md);
                }

                Kind::Tree => {
                    let tree_idx = metadata_graph.get_tree_idx(oid);
                    let tree_ref_iter =
                        unwrap_or_continue!(odb.find_tree_iter(&oid, &mut scratch), |e| {
                            warn!("Failed to find tree {oid}: {e}");
                        });
                    for child in tree_ref_iter {
                        let child = unwrap_or_continue!(child, |e| {
                            warn!("Failed to decode entry in tree {oid}: {e}");
                        });
                        use gix::objs::tree::EntryKind;
                        let child_idx = match child.mode.kind() {
                            EntryKind::Tree => metadata_graph.get_tree_idx(child.oid.into()),
                            EntryKind::Blob | EntryKind::BlobExecutable => {
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

        debug!("Built metadata graph in {:.6}s", t1.elapsed().as_secs_f64());

        let path = self.path.to_owned();
        match metadata_graph.get_repo_metadata() {
            Err(e) => {
                warn!("Failed to compute reachable blobs; ignoring metadata: {e}");
                let blobs = blobs
                    .into_iter()
                    .map(|(blob_oid, num_bytes)| BlobMetadata {
                        blob_oid,
                        num_bytes,
                        first_seen: Default::default(),
                    })
                    .collect();
                Ok(GitRepoResult {
                    repository: self.repo,
                    path,
                    blobs,
                    commit_metadata,
                })
            }
            Ok(md) => {
                let mut inverted =
                    HashMap::<ObjectId, SmallVec<[BlobAppearance; 1]>>::with_capacity_and_hasher(
                        num_blobs,
                        Default::default(),
                    );
                for e in md.into_iter() {
                    for (blob_oid, path) in e.introduced_blobs.into_iter() {
                        let vals = inverted.entry(blob_oid).or_insert(SmallVec::new());
                        vals.push(BlobAppearance {
                            commit_oid: e.commit_oid,
                            path,
                        });
                    }
                }

                // Build blobs result set.
                //
                // Apply any path-based ignore rules to blobs here, like the filesystem enumerator,
                // filtering out blobs that have paths to ignore. Note that the behavior of
                // ignoring blobs from Git repositories may be surprising:
                //
                // A blob may appear within a Git repository under many different paths.
                // Nosey Parker doesn't compute the *entire* set of paths that each blob
                // appears with. Instead, Nosey Parker computes the set of paths that each blob was
                // *first introduced* with.
                //
                // It is also possible to instruct Nosey Parker to compute *no* path information
                // for Git history.
                //
                // It's also possible (though rare) that a blob appears in a Git repository with
                // _no_ path whatsoever.
                //
                // Anyway, when Nosey Parker is determining whether a blob should be gitignored or
                // not, the logic is this:
                //
                // - If the set of pathnames for a blob is empty, *do not* ignore the blob.
                //
                // - If the set of pathnames for a blob is *not* empty, if *all* of the pathnames
                //   match the gitignore rules, ignore the blob.

                let blobs: Vec<_> = blobs
                    .into_iter()
                    .filter_map(|(blob_oid, num_bytes)| match inverted.get(&blob_oid) {
                        None => Some(BlobMetadata {
                            blob_oid,
                            num_bytes,
                            first_seen: SmallVec::new(),
                        }),

                        Some(first_seen) => {
                            let first_seen: SmallVec<_> = first_seen
                                .iter()
                                .filter(|entry| {
                                    use bstr::ByteSlice;
                                    match entry.path.to_path() {
                                        Ok(path) => {
                                            let is_dir = false;
                                            let m = self.gitignore.matched(path, is_dir);
                                            let is_ignore = m.is_ignore();
                                            // if is_ignore {
                                            //     debug!("ignoring path {}: {m:?}", path.display());
                                            // }
                                            !is_ignore
                                        }
                                        Err(_e) => {
                                            // debug!("error converting to path: {e}");
                                            true
                                        }
                                    }
                                })
                                .cloned()
                                .collect();

                            if first_seen.is_empty() {
                                // warn!("ignoring blob {blob_oid}");
                                None
                            } else {
                                Some(BlobMetadata {
                                    blob_oid,
                                    num_bytes,
                                    first_seen,
                                })
                            }
                        }
                    })
                    .collect();

                Ok(GitRepoResult {
                    repository: self.repo,
                    path,
                    blobs,
                    commit_metadata,
                })
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// git repo enumerator, sans metadata
// -------------------------------------------------------------------------------------------------
pub struct GitRepoEnumerator<'a> {
    path: &'a Path,
    repo: Repository,
}

impl<'a> GitRepoEnumerator<'a> {
    pub fn new(path: &'a Path, repo: Repository) -> Self {
        Self { path, repo }
    }

    pub fn run(self) -> Result<GitRepoResult> {
        use gix::object::Kind;
        use gix::odb::store::iter::Ordering;
        use gix::prelude::*;

        let _span = debug_span!("enumerate_git", "{}", self.path.display()).entered();

        let odb = &self.repo.objects;

        let mut blobs: Vec<(ObjectId, u64)> = Vec::with_capacity(64 * 1024);

        for oid in odb
            .iter()
            .context("Failed to iterate object database")?
            .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
        {
            let oid = unwrap_or_continue!(oid, |e| warn!("Failed to read object id: {e}"));
            let hdr = unwrap_or_continue!(odb.header(oid), |e| warn!(
                "Failed to read object header for {oid}: {e}"
            ));
            if hdr.kind() == Kind::Blob {
                let obj_size = hdr.size();
                blobs.push((oid, obj_size));
            }
        }

        let path = self.path.to_owned();
        let blobs = blobs
            .into_iter()
            .map(|(blob_oid, num_bytes)| BlobMetadata {
                blob_oid,
                num_bytes,
                first_seen: Default::default(),
            })
            .collect();
        Ok(GitRepoResult {
            repository: self.repo,
            path,
            blobs,
            commit_metadata: Default::default(),
        })
    }
}
