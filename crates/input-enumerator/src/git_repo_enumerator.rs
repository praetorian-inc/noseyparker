use anyhow::{Context, Result};
use gix::{hashtable::HashMap, ObjectId, OdbHandle, Repository};
use smallvec::SmallVec;
use std::path::{Path, PathBuf};
// use std::time::Instant;
use tracing::{error_span, warn};

use crate::blob_appearance::{BlobAppearance, BlobAppearanceSet};
use crate::git_commit_metadata::CommitMetadata;
use crate::git_metadata_graph::GitMetadataGraph;

use progress::Progress;

// -------------------------------------------------------------------------------------------------
// implementation helpers
// -------------------------------------------------------------------------------------------------
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
// FIXME: if keeping the pre-counting step, add some new kind of progress indicator
fn count_git_objects(odb: &OdbHandle, progress: &Progress) -> Result<ObjectCounts> {
    use gix::object::Kind;
    use gix::odb::store::iter::Ordering;
    use gix::prelude::*;

    let mut num_commits = 0;
    let mut num_trees = 0;
    let mut num_blobs = 0;

    for oid in odb
        .iter()
        .context("Failed to iterate object database")?
        .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
    {
        let oid = unwrap_or_continue!(oid, |e| {
            progress.suspend(|| warn!("Failed to read object id: {e}"))
        });
        let hdr = unwrap_or_continue!(odb.header(oid), |e| {
            progress.suspend(|| warn!("Failed to read object header for {oid}: {e}"))
        });
        match hdr.kind() {
            Kind::Commit => num_commits += 1,
            Kind::Tree => num_trees += 1,
            Kind::Blob => num_blobs += 1,
            Kind::Tag => {}
        }
    }
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
    repo: &'a Repository,
}

impl<'a> GitRepoWithMetadataEnumerator<'a> {
    pub fn new(path: &'a Path, repo: &'a Repository) -> Self {
        Self { path, repo }
    }

    pub fn run(&self, progress: &mut Progress) -> Result<GitRepoResult> {
        use gix::object::Kind;
        use gix::odb::store::iter::Ordering;
        use gix::prelude::*;

        let _span = error_span!("enumerate_git_with_metadata", "{}", self.path.display()).entered();

        macro_rules! warn {
            ($($arg:expr),*) => {
                progress.suspend(|| {
                    tracing::warn!($($arg),*);
                })
            }
        }

        let odb = &self.repo.objects;

        // TODO: measure how helpful or pointless it is to count the objects in advance
        // FIXME: if keeping the pre-counting step, add some new kind of progress indicator

        // First count the objects to figure out how big to allocate data structures.
        // We're assuming that the repository doesn't change in the meantime.
        // If it does, our allocation estimates won't be right. Too bad!
        let ObjectCounts {
            num_commits,
            num_trees,
            num_blobs,
        } = count_git_objects(odb, progress)?;

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
                    progress.inc(obj_size);
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

        let path = self.path.to_owned();
        match metadata_graph.repo_metadata(progress) {
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
                Ok(GitRepoResult {
                    path,
                    blobs,
                    commit_metadata,
                })
            }
            Ok(md) => {
                // FIXME: apply path-based ignore rules to blobs here, like the filesystem enumerator
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

                let blobs = blobs
                    .into_iter()
                    .map(|(blob_oid, num_bytes)| {
                        let first_seen = inverted
                            .get(&blob_oid)
                            .map_or(SmallVec::new(), |v| v.clone());
                        BlobMetadata {
                            blob_oid,
                            num_bytes,
                            first_seen,
                        }
                    })
                    .collect();
                Ok(GitRepoResult {
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
    repo: &'a Repository,
}

impl<'a> GitRepoEnumerator<'a> {
    pub fn new(path: &'a Path, repo: &'a Repository) -> Self {
        Self { path, repo }
    }

    pub fn run(&self, progress: &mut Progress) -> Result<GitRepoResult> {
        use gix::object::Kind;
        use gix::odb::store::iter::Ordering;
        use gix::prelude::*;

        let _span = error_span!("enumerate_git", "{}", self.path.display()).entered();

        macro_rules! warn {
            ($($arg:expr),*) => {
                progress.suspend(|| {
                    tracing::warn!($($arg),*);
                })
            }
        }

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
                progress.inc(obj_size);
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
            path,
            blobs,
            commit_metadata: Default::default(),
        })
    }
}
