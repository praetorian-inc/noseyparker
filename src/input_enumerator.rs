use anyhow::{bail, Result};
use git2::{Oid, Repository, RepositoryOpenFlags};
use ignore::{WalkBuilder, WalkState};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{debug, error, warn};

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

#[derive(Clone)]
pub struct FileResult {
    pub path: PathBuf,
    pub num_bytes: u64,
}

#[derive(Clone)]
pub struct GitRepoResult {
    pub path: PathBuf,
    pub blobs: Vec<(Oid, u64)>,
}

impl GitRepoResult {
    pub fn total_blob_bytes(&self) -> u64 {
        self.blobs.iter().map(|t| t.1).sum()
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
        /*
        debug!(
            "dropping! {} files, {} repos",
            self.local_files.len(),
            self.local_git_repos.len()
        );
        */
        self.global_files
            .lock()
            .unwrap()
            .extend(self.local_files.iter().cloned());
        self.global_git_repos
            .lock()
            .unwrap()
            .extend(self.local_git_repos.iter().cloned());
    }
}

impl<'t> ignore::ParallelVisitor for Visitor<'t> {
    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
        // FIXME: dedupe based on (device, inode) on platforms where available

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
                error!("Failed to get metadata for {:?}: {}; skipping", path, e);
                return WalkState::Skip;
            }
            Ok(v) => v,
        };

        if metadata.is_file() {
            let bytes = metadata.len();
            let path = path.to_owned();
            if self.file_too_big(bytes) {
                debug!("Skipping {:?}: size {} exceeds max size", &path, bytes);
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
                    error!("Failed to open Git repository at {:?}: {}; skipping", path, e);
                    return WalkState::Skip;
                }
                Ok(Some(repository)) => {
                    let enumerator = GitRepoEnumerator::new(&repository);
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
        }
        else {
            warn!("Unhandled path type: {:?}: {:?}", path, entry.file_type());
        }
        WalkState::Continue
    }
}

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

    pub fn new<T: AsRef<Path>>(inputs: &[T]) -> Result<Self> {
        if inputs.is_empty() {
            bail!("No inputs provided");
        }

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

    pub fn threads(&mut self, threads: usize) -> &mut Self {
        self.walk_builder.threads(threads);
        self
    }

    pub fn add_ignore<T: AsRef<Path>>(&mut self, path: T) -> Result<&mut Self> {
        match self.walk_builder.add_ignore(path) {
            Some(e) => Err(e)?,
            None => Ok(self),
        }
    }

    pub fn follow_links(&mut self, follow_links: bool) -> &mut Self {
        self.walk_builder.follow_links(follow_links);
        self
    }

    pub fn max_filesize(&mut self, max_filesize: Option<u64>) -> &mut Self {
        self.walk_builder.max_filesize(max_filesize);
        self.max_file_size = max_filesize;
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
pub fn open_git_repo(path: &Path) -> Result<Option<Repository>> {
    match Repository::open_ext(
        path,
        RepositoryOpenFlags::NO_SEARCH | RepositoryOpenFlags::NO_DOTGIT, // | RepositoryOpenFlags::BARE,
        &[] as &[&OsStr],
    ) {
        Err(e) => match e.code() {
            git2::ErrorCode::NotFound => Ok(None),
            _ => Err(e)?,
        },
        Ok(r) => Ok(Some(r)),
    }
}

pub struct GitRepoEnumeratorResult {
    pub blobs: Vec<(Oid, u64)>,
}

pub struct GitRepoEnumerator<'a> {
    repo: &'a Repository,
}

impl<'a> GitRepoEnumerator<'a> {
    pub fn new(repo: &'a Repository) -> Self {
        GitRepoEnumerator { repo }
    }

    pub fn run(&self, progress: &mut Progress) -> Result<GitRepoEnumeratorResult> {
        let mut blobs: Vec<(Oid, u64)> = Vec::new();

        let odb = self.repo.odb()?;
        odb.foreach(|oid: &git2::Oid| {
            let (obj_size, obj_type) = match odb.read_header(*oid) {
                Err(e) => {
                    error!("Failed to read object header {}: {}", oid, e);
                    return true;
                }
                Ok(v) => v,
            };
            if obj_type == git2::ObjectType::Blob {
                let obj_size = obj_size as u64;
                progress.inc(obj_size);
                blobs.push((*oid, obj_size));
            }
            true
        })?;

        Ok(GitRepoEnumeratorResult { blobs })
    }
}
