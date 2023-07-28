use anyhow::{Context, Result};
use gix::{ObjectId, Repository, hashtable::HashMap};
use smallvec::SmallVec;
use std::path::{Path, PathBuf};
// use std::time::Instant;
use tracing::{debug, error, error_span, info, warn};

use crate::blob_appearance::{BlobAppearance, BlobAppearanceSet};
use crate::git_metadata_graph::GitMetadataGraph;
use crate::progress::Progress;


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


#[derive(Clone)]
pub struct BlobMetadata {
    pub blob_oid: ObjectId,
    pub num_bytes: u64,
    pub first_seen: BlobAppearanceSet,
}


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

        let _span = error_span!("git_enumerator", "{}", self.path.display()).entered();

        macro_rules! warn {
            ($($arg:expr),*) => {
                progress.suspend(|| {
                    tracing::warn!($($arg),*);
                })
            }
        }

        /*
        macro_rules! info {
            ($($arg:expr),*) => {
                progress.suspend(|| {
                    tracing::info!($($arg),*);
                })
            }
        }
        */

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
        // let t1 = Instant::now();
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
                Ok(GitRepoResult { path, blobs })
            }
            Ok(md) => {
                // FIXME: apply path-based ignore rules to blobs here, like the filesystem enumerator
                let mut inverted = HashMap::<ObjectId, SmallVec<[BlobAppearance; 1]>>::with_capacity_and_hasher(num_blobs, Default::default());
                for e in md.into_iter() {
                    for (blob_oid, path) in e.introduced_blobs.into_iter() {
                        let vals = inverted.entry(blob_oid).or_insert(SmallVec::new());
                        vals.push(BlobAppearance{commit_oid: e.commit_oid, path });
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
