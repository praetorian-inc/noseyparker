pub mod blob_appearance;
pub mod bstring_table;
pub mod git_commit_metadata;
pub mod git_metadata_graph;
pub use gix::{Repository, ThreadSafeRepository};

use anyhow::{bail, Result};
use crossbeam_channel::Sender;
pub use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{DirEntry, WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

// -------------------------------------------------------------------------------------------------
// helper macros
// -------------------------------------------------------------------------------------------------
macro_rules! unwrap_some_or_continue {
    ($arg:expr, $on_error:expr $(,)?) => {
        match $arg {
            Some(v) => v,
            None => {
                #[allow(clippy::redundant_closure_call)]
                $on_error();
                continue;
            }
        }
    };
}

pub(crate) use unwrap_some_or_continue;

macro_rules! unwrap_ok_or_continue {
    ($arg:expr, $on_error:expr $(,)?) => {
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

pub(crate) use unwrap_ok_or_continue;

// -------------------------------------------------------------------------------------------------
mod git_repo_enumerator;
pub use git_repo_enumerator::{GitRepoEnumerator, GitRepoResult, GitRepoWithMetadataEnumerator};

pub enum FoundInput {
    File(FileResult),
    Directory(DirectoryResult),
    EnumeratorFile(EnumeratorFileResult),
}

pub struct FileResult {
    pub path: PathBuf,
    pub num_bytes: u64,
}

pub struct EnumeratorFileResult {
    pub path: PathBuf,
}

pub struct DirectoryResult {
    pub path: PathBuf,
}

pub type Output = Sender<FoundInput>;

// -------------------------------------------------------------------------------------------------
// VisitorBuilder
// -------------------------------------------------------------------------------------------------
struct VisitorBuilder<'t> {
    max_file_size: Option<u64>,
    output: &'t Output,
}

impl<'s, 't> ignore::ParallelVisitorBuilder<'s> for VisitorBuilder<'t>
where
    't: 's,
{
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(Visitor {
            max_file_size: self.max_file_size,
            output: self.output,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Visitor
// -------------------------------------------------------------------------------------------------
struct Visitor<'t> {
    max_file_size: Option<u64>,
    output: &'t Output,
}

impl<'t> Visitor<'t> {
    #[inline]
    fn file_too_big(&self, size: u64) -> bool {
        self.max_file_size.map_or(false, |max_size| size > max_size)
    }

    fn found_file(&mut self, r: FileResult) {
        self.output.send(FoundInput::File(r)).unwrap();
    }

    fn found_directory(&mut self, r: DirectoryResult) {
        self.output.send(FoundInput::Directory(r)).unwrap();
    }
}

impl<'t> ignore::ParallelVisitor for Visitor<'t> {
    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
        // FIXME: dedupe based on (device, inode) on platforms where available; see https://docs.rs/same-file/1.0.6/same_file/ for ideas

        let entry = match result {
            Err(e) => {
                warn!("Skipping entry: {e}");
                return WalkState::Skip;
            }
            Ok(v) => v,
        };

        let path = entry.path();
        let metadata = match entry.metadata() {
            Err(e) => {
                warn!("Skipping {}: failed to get metadata: {e}", path.display());
                return WalkState::Skip;
            }
            Ok(v) => v,
        };

        if metadata.is_file() {
            let num_bytes = metadata.len();
            if self.file_too_big(num_bytes) {
                debug!("Skipping {}: size {num_bytes} exceeds max size", path.display());
            } else {
                let path = path.to_owned();
                self.found_file(FileResult { path, num_bytes });
            }
        } else if metadata.is_dir() {
            // Skip things that look like Nosey Parker datastores
            if path.join("datastore.db").is_file()
                && path.join("scratch").is_dir()
                && path.join("clones").is_dir()
                && path.join("blobs").is_dir()
            {
                debug!("Skipping {}: looks like a Nosey Parker datastore", path.display());
                return WalkState::Skip;
            } else {
                self.found_directory(DirectoryResult {
                    path: path.to_owned(),
                });
            }
        } else if metadata.is_symlink() {
            // No problem; just ignore it
            //
            // Had follow_symlinks been enabled, the pointed-to entry would have been yielded
            // instead.
        } else {
            debug!("Skipping {}: unhandled path type: {:?}", path.display(), entry.file_type());
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
    /// The inner filesystem walker builder
    walk_builder: WalkBuilder,

    /// A gitignore builder, used for propagating path-based ignore rules into git history enumeration
    ///
    /// Note that this is a duplicate representation of the gitignore rules stored within
    /// `walk_builder`. There seems to be no avoiding that with the APIs exposed by the
    /// `WalkBuilder` type today.
    gitignore_builder: GitignoreBuilder,

    /// We store the max file size here in addition to inside the `walk_builder` to work around a
    /// bug in `ignore` where max filesize is not applied to top-level file inputs, only inputs that
    /// appear under a directory.
    max_file_size: Option<u64>,

    /// Should git metadata (commit and path information) be collected?
    collect_git_metadata: bool,

    /// Should git history be scanned at all?
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
            collect_git_metadata: Self::DEFAULT_COLLECT_GIT_METADATA,
            enumerate_git_history: Self::DEFAULT_ENUMERATE_GIT_HISTORY,
            gitignore_builder: GitignoreBuilder::new(""),
        })
    }

    /// Set the number of parallel enumeration threads.
    pub fn threads(&mut self, threads: usize) -> &mut Self {
        self.walk_builder.threads(threads);
        self
    }

    /// Add a set of gitignore-style rules from the given ignore file.
    pub fn add_ignore<T: AsRef<Path>>(&mut self, path: T) -> Result<&mut Self> {
        let path = path.as_ref();

        if let Some(e) = self.gitignore_builder.add(path) {
            Err(e)?;
        }

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
    /// Only entries that satisfy the predicate will be enumerated.
    ///
    /// This can be used to skip entire directories.
    pub fn filter_entry<P>(&mut self, filter: P) -> &mut Self
    where
        P: Fn(&DirEntry) -> bool + Send + Sync + 'static,
    {
        self.walk_builder.filter_entry(filter);
        self
    }

    /// Get the configured Gitignore for this enumerator.
    pub fn gitignore(&self) -> Result<Gitignore> {
        Ok(self.gitignore_builder.build()?)
    }

    pub fn run(&self, output: Output) -> Result<()> {
        let mut visitor_builder = VisitorBuilder {
            max_file_size: self.max_file_size,
            output: &output,
        };

        self.walk_builder
            .build_parallel()
            .visit(&mut visitor_builder);

        Ok(())
    }
}

/// Opens the given Git repository if it exists, returning None otherwise.
pub fn open_git_repo(path: &Path) -> Result<Option<Repository>> {
    let opts = gix::open::Options::isolated().open_path_as_is(true);
    match gix::open_opts(path, opts) {
        Err(gix::open::Error::NotARepository { .. }) => Ok(None),
        Err(err) => Err(err.into()),
        Ok(repo) => Ok(Some(repo)),
    }
}
