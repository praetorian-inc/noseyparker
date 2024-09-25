use anyhow::{Context, Result};
use gix::{hashtable::HashMap, ObjectId, Repository};
use ignore::gitignore::Gitignore;
use smallvec::SmallVec;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
// use std::time::Instant;
use tracing::{debug, debug_span, error};

use crate::blob_appearance::{BlobAppearance, BlobAppearanceSet};
use crate::git_commit_metadata::CommitMetadata;
use crate::git_metadata_graph::{GitMetadataGraph, ObjectIndex};
use crate::{unwrap_ok_or_continue, unwrap_some_or_continue};

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
    pub commit_metadata: HashMap<ObjectId, Arc<CommitMetadata>>,
}

#[derive(Clone)]
pub struct BlobMetadata {
    pub blob_oid: ObjectId,
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
        use gix::prelude::*;

        let _span = debug_span!("enumerate_git_with_metadata", "{}", self.path.display()).entered();

        let odb = &self.repo.objects;

        // First count the objects to figure out how big to allocate data structures.
        // We're assuming that the repository doesn't change in the meantime.
        // If it does, our allocation estimates won't be right. Too bad!
        let object_index = ObjectIndex::new(odb)?;
        debug!(
            "Indexed {} objects in {:.6}s; {} blobs; {} commits",
            object_index.num_objects(),
            t1.elapsed().as_secs_f64(),
            object_index.num_blobs(),
            object_index.num_commits(),
        );

        let mut blobs: Vec<ObjectId> = Vec::with_capacity(object_index.num_blobs());
        let mut metadata_graph = GitMetadataGraph::with_capacity(object_index.num_commits());

        // scratch buffer used for decoding commits.
        // size chosen here based on experimentation: biggest commit/tree in cpython is ~250KiB
        // choose a big enough size to avoid resizing in almost all cases
        let mut scratch: Vec<u8> = Vec::with_capacity(4 * 1024 * 1024);

        let mut commit_metadata =
            HashMap::with_capacity_and_hasher(object_index.num_commits(), Default::default());

        for &(oid, kind) in object_index.idx_to_oid_and_md.iter() {
            match kind {
                Kind::Blob => blobs.push(oid),

                Kind::Commit => {
                    let commit = unwrap_ok_or_continue!(odb.find_commit(&oid, &mut scratch), |e| {
                        error!("Failed to find commit {oid}: {e}");
                    });

                    let tree_oid = commit.tree();
                    let tree_idx =
                        unwrap_some_or_continue!(object_index.get_index(&tree_oid), || {
                            error!("Failed to find tree {tree_oid} for commit {oid}");
                        });
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
                    commit_metadata.insert(oid, Arc::new(md));
                }

                Kind::Tree => {}

                Kind::Tag => {}
            }
        }

        debug!("Built metadata graph in {:.6}s", t1.elapsed().as_secs_f64());

        match metadata_graph.get_repo_metadata(&object_index, &self.repo) {
            Err(e) => {
                error!("Failed to compute reachable blobs; ignoring metadata: {e}");
                let blobs = blobs
                    .into_iter()
                    .map(|blob_oid| BlobMetadata {
                        blob_oid,
                        first_seen: Default::default(),
                    })
                    .collect();
                Ok(GitRepoResult {
                    repository: self.repo,
                    path: self.path.to_owned(),
                    blobs,
                    commit_metadata,
                })
            }
            Ok(md) => {
                let mut blob_to_appearance =
                    HashMap::<ObjectId, SmallVec<[BlobAppearance; 1]>>::with_capacity_and_hasher(
                        object_index.num_blobs(),
                        Default::default(),
                    );
                for e in md.into_iter() {
                    for (blob_oid, path) in e.introduced_blobs.into_iter() {
                        let vals = blob_to_appearance
                            .entry(blob_oid)
                            .or_insert(SmallVec::new());
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
                    .filter_map(|blob_oid| match blob_to_appearance.get(&blob_oid) {
                        None => Some(BlobMetadata {
                            blob_oid,
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
                                    first_seen,
                                })
                            }
                        }
                    })
                    .collect();

                Ok(GitRepoResult {
                    repository: self.repo,
                    path: self.path.to_owned(),
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

        let mut blobs: Vec<ObjectId> = Vec::with_capacity(64 * 1024);

        for oid in odb
            .iter()
            .context("Failed to iterate object database")?
            .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
        {
            let oid = unwrap_ok_or_continue!(oid, |e| error!("Failed to read object id: {e}"));
            let hdr = unwrap_ok_or_continue!(odb.header(oid), |e| error!(
                "Failed to read object header for {oid}: {e}"
            ));
            if hdr.kind() == Kind::Blob {
                blobs.push(oid);
            }
        }

        let blobs = blobs
            .into_iter()
            .map(|blob_oid| BlobMetadata {
                blob_oid,
                first_seen: Default::default(),
            })
            .collect();
        Ok(GitRepoResult {
            repository: self.repo,
            path: self.path.to_owned(),
            blobs,
            commit_metadata: Default::default(),
        })
    }
}
