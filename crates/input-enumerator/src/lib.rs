pub mod blob_appearance;
pub mod bstring_table;
pub mod git_commit_metadata;
pub mod git_metadata_graph;

use anyhow::Result;
use ignore::{DirEntry, WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{debug, error, warn};

use progress::Progress;

mod git_repo_enumerator;
pub use git_repo_enumerator::{GitRepoEnumerator, GitRepoResult, GitRepoWithMetadataEnumerator};

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

// -------------------------------------------------------------------------------------------------
// VisitorBuilder
// -------------------------------------------------------------------------------------------------
struct VisitorBuilder<'t> {
    max_file_size: Option<u64>,
    collect_git_metadata: bool,
    enumerate_git_history: bool,

    global_files: &'t Mutex<Vec<FileResult>>,
    global_git_repos: &'t Mutex<Vec<GitRepoResult>>,

    progress: &'t Progress,
}

impl<'s, 't> ignore::ParallelVisitorBuilder<'s> for VisitorBuilder<'t>
where
    't: 's,
{
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(Visitor {
            max_file_size: self.max_file_size,
            collect_git_metadata: self.collect_git_metadata,
            enumerate_git_history: self.enumerate_git_history,
            local_files: Vec::new(),
            local_git_repos: Vec::new(),
            global_files: self.global_files,
            global_git_repos: self.global_git_repos,
            progress: self.progress.clone(),
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Visitor
// -------------------------------------------------------------------------------------------------
struct Visitor<'t> {
    collect_git_metadata: bool,
    enumerate_git_history: bool,
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
                error!("Failed to get metadata for {}: {e}; skipping", path.display());
                return WalkState::Skip;
            }
            Ok(v) => v,
        };
        let is_dir = metadata.is_dir();

        if metadata.is_file() {
            let bytes = metadata.len();
            let path = path.to_owned();
            if self.file_too_big(bytes) {
                debug!("Skipping {}: size {bytes} exceeds max size", path.display());
            } else {
                self.progress.inc(bytes);
                self.local_files.push(FileResult {
                    path,
                    num_bytes: bytes,
                });
            }
        } else if is_dir {
            if self.enumerate_git_history {
                match open_git_repo(path) {
                    Err(e) => {
                        error!(
                            "Failed to open Git repository at {}: {e}; skipping",
                            path.display()
                        );
                        return WalkState::Skip;
                    }
                    Ok(Some(repository)) => {
                        debug!("Found Git repo at {}", path.display());

                        if self.collect_git_metadata {
                            let enumerator = GitRepoWithMetadataEnumerator::new(path, &repository);
                            match enumerator.run(&mut self.progress) {
                                Err(e) => {
                                    error!(
                                        "Failed to enumerate Git repository at {}: {e}; skipping",
                                        path.display()
                                    );
                                    return WalkState::Skip;
                                }
                                Ok(r) => self.local_git_repos.push(r),
                            }
                        } else {
                            let enumerator = GitRepoEnumerator::new(path, &repository);
                            match enumerator.run(&mut self.progress) {
                                Err(e) => {
                                    error!(
                                        "Failed to enumerate Git repository at {}: {e}; skipping",
                                        path.display()
                                    );
                                    return WalkState::Skip;
                                }
                                Ok(r) => self.local_git_repos.push(r),
                            }
                        }
                    }
                    Ok(None) => {}
                }
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

    collect_git_metadata: bool,
    enumerate_git_history: bool,
}

impl FilesystemEnumerator {
    pub const DEFAULT_MAX_FILESIZE: u64 = 100 * 1024 * 1024;
    pub const DEFAULT_FOLLOW_LINKS: bool = false;
    pub const DEFAULT_COLLECT_GIT_METADATA: bool = true;
    pub const DEFAULT_ENUMERATE_GIT_HISTORY: bool = true;

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
            collect_git_metadata: Self::DEFAULT_COLLECT_GIT_METADATA,
            enumerate_git_history: Self::DEFAULT_ENUMERATE_GIT_HISTORY,
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

    /// Enable or disable whether detailed Git metadata will be collected.
    pub fn collect_git_metadata(&mut self, collect_git_metadata: bool) -> &mut Self {
        self.collect_git_metadata = collect_git_metadata;
        self
    }

    /// Enable or disable whether Git history is enumerated.
    pub fn enumerate_git_history(&mut self, enumerate_git_history: bool) -> &mut Self {
        self.enumerate_git_history = enumerate_git_history;
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
            collect_git_metadata: self.collect_git_metadata,
            enumerate_git_history: self.enumerate_git_history,
            max_file_size: self.max_file_size,
            global_files: &files,
            global_git_repos: &git_repos,
            progress,
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
