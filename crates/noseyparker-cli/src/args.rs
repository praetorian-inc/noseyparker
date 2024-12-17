//! Nosey Parker's command-line interface is specified here in one monolithic file.
//!
//! The command-line interface is defined using `clap`.

use clap::{
    crate_description, crate_version, ArgAction, Args, Parser, Subcommand, ValueEnum, ValueHint,
};
use lazy_static::lazy_static;
use noseyparker::git_url::GitUrl;
use std::io::IsTerminal;
use std::path::PathBuf;
use strum::Display;
#[cfg(feature = "github")]
use url::Url;

use crate::util::get_writer_for_file_or_stdout;

// -----------------------------------------------------------------------------
// system information
// -----------------------------------------------------------------------------
lazy_static! {
    /// How much RAM is installed in the system?
    static ref RAM_GB: Option<f64> = {
        if sysinfo::IS_SUPPORTED_SYSTEM {
            use sysinfo::{System,RefreshKind,MemoryRefreshKind};
            let s = System::new_with_specifics(RefreshKind::default().with_memory(MemoryRefreshKind::default().with_ram()));
            Some(s.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0)
        } else {
            None
        }
    };
}

// -----------------------------------------------------------------------------
// utilities
// -----------------------------------------------------------------------------
#[rustfmt::skip]
fn get_long_version() -> &'static str {
    concat!(
        crate_version!(),
        "\n",
        "\n", "Build Configuration:",
        "\n",
        "\n", "    Build Timestamp:    ", env!("VERGEN_BUILD_TIMESTAMP"),
        "\n",
        "\n", "    Commit Timestamp:   ", env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        "\n", "    Commit Branch:      ", env!("VERGEN_GIT_BRANCH"),
        "\n", "    Commit SHA:         ", env!("VERGEN_GIT_SHA"),
        "\n",
        "\n", "    Cargo Features:     ", env!("VERGEN_CARGO_FEATURES"),
        "\n", "    Debug:              ", env!("VERGEN_CARGO_DEBUG"),
        "\n", "    Optimization:       ", env!("VERGEN_CARGO_OPT_LEVEL"),
        "\n", "    Target Triple:      ", env!("VERGEN_CARGO_TARGET_TRIPLE"),
        "\n",
        "\n", "Build System:",
        "\n",
        "\n", "    OS:                 ", env!("VERGEN_SYSINFO_NAME"),
        "\n", "    OS Version:         ", env!("VERGEN_SYSINFO_OS_VERSION"),
        "\n",
        "\n", "    CPU Vendor:         ", env!("VERGEN_SYSINFO_CPU_VENDOR"),
        "\n", "    CPU Brand:          ", env!("VERGEN_SYSINFO_CPU_BRAND"),
        "\n", "    CPU Cores:          ", env!("VERGEN_SYSINFO_CPU_CORE_COUNT"),
        "\n",
        "\n", "    rustc Version:      ", env!("VERGEN_RUSTC_SEMVER"),
        "\n", "    rustc Channel:      ", env!("VERGEN_RUSTC_CHANNEL"),
        "\n", "    rustc Host Triple:  ", env!("VERGEN_RUSTC_HOST_TRIPLE"),
        "\n", "    rustc Commit Date:  ", env!("VERGEN_RUSTC_COMMIT_DATE"),
        "\n", "    rustc Commit SHA:   ", env!("VERGEN_RUSTC_COMMIT_HASH"),
        "\n", "    rustc LLVM Version: ", env!("VERGEN_RUSTC_LLVM_VERSION"),
    )
}

/// Get a filename-friendly short version string, suitable for naming a release archive
fn get_short_version() -> &'static str {
    concat!("v", clap::crate_version!(), "-", env!("VERGEN_CARGO_TARGET_TRIPLE"),)
}

const DEFAULT_DATASTORE: &str = "datastore.np";

#[cfg(feature = "github")]
pub fn validate_github_api_url(github_api_url: &Url, all_github_organizations: bool) {
    use clap::error::ErrorKind;
    use clap::CommandFactory;

    // Check that a non-default value of `--github-api-url` has been specified.
    // This constraint is impossible to express natively using `clap` version 4.
    if let Some(host) = github_api_url.host_str() {
        if host == "api.github.com" && all_github_organizations {
            let mut cmd = CommandLineArgs::command();
            let err = cmd.error(
                ErrorKind::MissingRequiredArgument,
                "a non-default value for `--github-api-url` is required when using `--all-github-organizations`",
            );
            err.exit();
        }
    }
}

/// How many parallel scan jobs should be used by default?
///
/// This is based on the number of available vCPUs, and also takes into account the amount of
/// memory per core.
fn default_scan_jobs() -> usize {
    match (std::thread::available_parallelism(), *RAM_GB) {
        (Ok(v), Some(ram_gb)) => {
            let n: usize = v.into();
            let max_n = (ram_gb / 3.0).ceil().max(1.0) as usize;
            n.clamp(1, max_n)
        }
        (Ok(v), None) => v.into(),
        (Err(_e), _) => 1,
    }
}

// -----------------------------------------------------------------------------
// command-line args
// -----------------------------------------------------------------------------
#[derive(Parser, Debug)]
#[command(
    name("noseyparker"),
    bin_name("noseyparker"),

    author,   // retrieved from Cargo.toml `authors`
    about,    // retrieved from Cargo.toml `description`

    version = get_short_version(),
    long_version = get_long_version(),

    long_about = concat!(
        crate_description!(),
    ),
)]
#[deny(missing_docs)]
/// Find secrets and sensitive information in textual data
pub struct CommandLineArgs {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    // FIXME: suppress from showing long help in subcommand help; only show on top-level `help`
    pub global_args: GlobalArgs,
}

impl CommandLineArgs {
    pub fn parse_args() -> Self {
        let mut cmd = <Self as clap::CommandFactory>::command();
        let matches = cmd.get_matches_mut();

        use clap::parser::ValueSource;

        // Make sure that if the `scan` command is specified and the default datastore is used,
        // that the datastore does not already exist.
        // See #74.
        if let Some(("scan", sub_matches)) = matches.subcommand() {
            let datastore_value: &PathBuf = sub_matches
                .get_one("datastore")
                .expect("datastore arg should be present");
            if let Some(ValueSource::DefaultValue) = sub_matches.value_source("datastore") {
                if datastore_value.exists() {
                    cmd.error(
                        clap::error::ErrorKind::InvalidValue,
                        format!(
                            "the default datastore at {} exists; \
                                       explicitly specify the datastore if you wish to update it",
                            datastore_value.display()
                        ),
                    )
                    .exit();
                }
            }
        }

        let mut args = match <Self as clap::FromArgMatches>::from_arg_matches(&matches) {
            Ok(args) => args,
            Err(e) => e.exit(),
        };

        // If `NO_COLOR` is set in the environment, disable colored output
        //
        // https://no-color.org/
        if std::env::var("NO_COLOR").is_ok() {
            args.global_args.color = Mode::Never
        }

        // If `--quiet` is specified, disable progress bars
        if args.global_args.quiet {
            args.global_args.progress = Mode::Never;
        }

        args
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[cfg(feature = "github")]
    /// Scan content for secrets
    ///
    /// This command uses regex-based rules to identify hardcoded secrets and other potentially sensitive information in textual content (or in inputs that can have textual content extracted from them).
    ///
    /// The findings from scanning are recorded into a datastore. The recorded findings can later
    /// be reported in several formats using the `summarize` and `report` commands.
    ///
    /// Several types of inputs can be specified:
    ///
    /// - Positional input arguments can be either files or directories.
    ///   Files are scanned directly; directories are recursively enumerated and scanned.
    ///   Any directories encountered that are Git repositories will have their entire history scanned.
    ///
    /// - A Git repository URL can be specified with the `--git-repo=URL` argument.
    ///   This will cause Nosey Parker to clone that repository to its datastore and scan its history.
    ///
    /// - A GitHub user can be specified with the `--github-user=NAME` argument.
    ///   This will cause Nosey Parker to enumerate accessible repositories belonging to that user, clone them to its datastore, and scan their entire history.
    ///
    /// - A GitHub organization can be specified with the `--github-org=NAME` argument.
    ///   This will cause Nosey Parker to enumerate accessible repositories belonging to that organization, clone them to its datastore, and scan their entire history.
    ///
    /// The `git` binary on the PATH is used to clone any required Git repositories.
    /// It is careful invoked to avoid using any system-wide or user-specific configuration.
    ///
    /// By default, when cloning repositories from GitHub or enumerating GitHub users or organizations, unauthenticated access is used.
    /// An optional personal access token can be specified using the `NP_GITHUB_TOKEN` environment variable.
    /// Using a personal access token gives higher rate limits and may make additional content accessible.
    #[command(display_order = 1)]
    Scan(ScanArgs),

    #[cfg(not(feature = "github"))]
    /// Scan content for secrets
    ///
    /// This command uses regex-based rules to identify hardcoded secrets and other potentially sensitive information in textual content (or in inputs that can have textual content extracted from them).
    ///
    /// The findings from scanning are recorded into a datastore. The recorded findings can later
    /// be reported in several formats using the `summarize` and `report` commands.
    ///
    /// Several types of inputs can be specified:
    ///
    /// - Positional input arguments can be either files or directories.
    ///   Files are scanned directly; directories are recursively enumerated and scanned.
    ///   Any directories encountered that are Git repositories will have their entire history scanned.
    ///
    /// - A Git repository URL can be specified with the `--git-repo=URL` argument.
    ///   This will cause Nosey Parker to clone that repository to its datastore and scan its history.
    ///
    /// The `git` binary on the PATH is used to clone any required Git repositories.
    /// It is careful invoked to avoid using any system-wide or user-specific configuration.
    #[command(display_order = 1)]
    Scan(ScanArgs),

    /// Summarize scan findings
    ///
    /// Findings are summarized in tabular form.
    /// The default `human` format prints a table of findings with one row for each rule that produced findings.
    /// The table has several columns:
    ///
    /// - Rule: the name of the rule
    ///
    /// - Findings: the number of findings, i.e., the number of distinct match group values produced by the rule
    ///
    /// - Matches: the number of individual matches
    ///
    /// - Accepted: the number of findings whose matches have `accept` status
    ///
    /// - Rejected: the number of findings whose matches have `reject` status
    ///
    /// - Mixed: the number of findings whose matches have a mix of `accept` and `reject` status
    ///
    /// - Unlabeled: the number of findings whose matches have no status at all
    #[command(display_order = 2, alias = "summarise")]
    Summarize(SummarizeArgs),

    /// Report detailed scan findings
    #[command(display_order = 3)]
    Report(ReportArgs),

    #[cfg(feature = "github")]
    /// Interact with GitHub
    ///
    /// By default, unauthenticated access is used.
    /// An optional personal access token can be specified using the `NP_GITHUB_TOKEN` environment variable.
    /// Using a personal access token gives higher rate limits and may make additional content accessible.
    #[command(display_order = 4, name = "github")]
    GitHub(GitHubArgs),

    /// Manage datastores
    #[command(display_order = 30)]
    Datastore(DatastoreArgs),

    /// Manage rules and rulesets
    #[command(display_order = 30, alias = "rule")]
    Rules(RulesArgs),

    /// Manage annotations (experimental)
    ///
    /// Annotations include assigned status (`accept` or `reject`) and freeform comments.
    #[command(display_order = 40)]
    Annotations(AnnotationsArgs),

    /// Generate Nosey Parker release assets
    ///
    /// This command is used primarily for generation of artifacts to be included in releases.
    #[command(display_order = 50)]
    Generate(GenerateArgs),
}

// -----------------------------------------------------------------------------
// global options
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
#[command(next_help_heading = "Global Options")]
pub struct GlobalArgs {
    /// Enable verbose output
    ///
    /// This can be repeated up to 3 times to enable successively more output.
    #[arg(global=true, long, short, action=ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error feedback messages
    ///
    /// This silences WARNING, INFO, DEBUG, and TRACE messages and disables progress bars.
    /// This overrides any provided verbosity and progress reporting options.
    #[arg(global = true, long, short)]
    pub quiet: bool,

    /// Enable or disable colored output
    ///
    /// When this is "auto", colors are enabled for stdout and stderr when they are terminals.
    ///
    /// If the `NO_COLOR` environment variable is set, it takes precedence and is equivalent to `--color=never`.
    #[arg(global=true, long, default_value_t=Mode::Auto, value_name="MODE", alias="colour")]
    pub color: Mode,

    /// Enable or disable progress bars
    ///
    /// When this is "auto", progress bars are enabled when stderr is a terminal.
    #[arg(global=true, long, default_value_t=Mode::Auto, value_name="MODE")]
    pub progress: Mode,

    /// Ignore validation of TLS certificates
    #[arg(global = true, long)]
    pub ignore_certs: bool,

    #[command(flatten)]
    pub advanced: AdvancedArgs,
}

#[derive(Args, Debug)]
#[command(next_help_heading = "Advanced Global Options")]
/// These are advanced options that should not need to be used in normal circumstances.
pub struct AdvancedArgs {
    /// Set the rlimit for number of open files to LIMIT
    ///
    /// This should not need to be changed from the default unless you run into crashes from
    /// running out of file descriptors.
    #[arg(
        hide_short_help = true,
        global = true,
        long,
        default_value_t = 16384,
        value_name = "LIMIT"
    )]
    pub rlimit_nofile: u64,

    /// Set the cache size for SQLite connections to SIZE
    ///
    /// This has the effect of setting SQLite's `pragma cache_size=SIZE`.
    /// The default value is set to use a maximum of 1GiB for database cache.
    /// See <https://sqlite.org/pragma.html#pragma_cache_size> for more details.
    #[arg(
        hide_short_help=true,
        global=true,
        long,
        default_value_t=-1 * 1024 * 1024,
        value_name="SIZE",
        allow_negative_numbers=true,
    )]
    pub sqlite_cache_size: i64,

    /// Enable or disable backtraces on panic
    ///
    /// This has the effect of setting the `RUST_BACKTRACE` environment variable to 1.
    #[arg(hide_short_help=true, global=true, long, default_value_t=true, action=ArgAction::Set, value_name="BOOL")]
    pub enable_backtraces: bool,
}

impl GlobalArgs {
    pub fn use_color<T: IsTerminal>(&self, out: T) -> bool {
        match self.color {
            Mode::Never => false,
            Mode::Always => true,
            Mode::Auto => out.is_terminal(),
        }
    }

    pub fn use_progress(&self) -> bool {
        match self.progress {
            Mode::Never => false,
            Mode::Always => true,
            Mode::Auto => std::io::stderr().is_terminal(),
        }
    }
}

/// A generic auto/never/always mode value
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum Mode {
    Auto,
    Never,
    Always,
}

// -----------------------------------------------------------------------------
// `github` command
// -----------------------------------------------------------------------------
#[cfg(feature = "github")]
#[derive(Args, Debug)]
pub struct GitHubArgs {
    #[command(subcommand)]
    pub command: GitHubCommand,

    /// Use the specified URL for GitHub API access
    ///
    /// If accessing a GitHub Enterprise Server instance, this value should be the entire base URL
    /// include the `api/v3` portion, e.g., `https://github.example.com/api/v3`.
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        default_value = "https://api.github.com/",
        visible_alias="api-url",
        global = true,
    )]
    pub github_api_url: Url,
}

#[cfg(feature = "github")]
#[derive(Subcommand, Debug)]
pub enum GitHubCommand {
    /// Interact with GitHub repositories
    #[command(subcommand)]
    Repos(GitHubReposCommand),
}

#[cfg(feature = "github")]
#[derive(Subcommand, Debug)]
pub enum GitHubReposCommand {
    /// List repositories belonging to a specific user or organization
    List(GitHubReposListArgs),
}

#[cfg(feature = "github")]
#[derive(Args, Debug)]
pub struct GitHubReposListArgs {
    #[command(flatten)]
    pub repo_specifiers: GitHubRepoSpecifiers,

    #[command(flatten)]
    pub output_args: OutputArgs<GitHubOutputFormat>,
}

#[cfg(feature = "github")]
#[derive(Args, Debug, Clone)]
#[command(next_help_heading = "Input Specifier Options")]
pub struct GitHubRepoSpecifiers {
    /// Select repositories belonging to the specified user
    ///
    /// This option can be repeated.
    #[arg(long, visible_alias = "github-user")]
    pub user: Vec<String>,

    /// Select repositories belonging to the specified organization
    ///
    /// This option can be repeated.
    #[arg(
        long,
        visible_alias = "org",
        visible_alias = "github-organization",
        visible_alias = "github-org"
    )]
    pub organization: Vec<String>,

    /// Select repositories belonging to all organizations
    ///
    /// This only works with a GitHub Enterprise Server instance.
    /// The `--github-api-url` option must be specified.
    #[arg(
        long,
        visible_alias = "all-orgs",
        visible_alias = "all-github-organizations",
        visible_alias = "all-github-orgs"
    )]
    pub all_organizations: bool,

    /// Select only GitHub repos of the given type
    #[arg(
        long,
        visible_alias = "github-repo-type",
        value_name="TYPE",
        default_value_t = GitHubRepoType::Source,
    )]
    pub repo_type: GitHubRepoType,
}

#[cfg(feature = "github")]
impl GitHubRepoSpecifiers {
    pub fn is_empty(&self) -> bool {
        self.user.is_empty() && self.organization.is_empty() && !self.all_organizations
    }
}

// -----------------------------------------------------------------------------
// `rules` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct RulesArgs {
    #[command(subcommand)]
    pub command: RulesCommand,
}

#[derive(Subcommand, Debug)]
pub enum RulesCommand {
    /// Check rules for problems
    ///
    /// If errors are detected or if warnings are detected and `--warnings-as-errors` is specified, the program will exit with a nonzero exit code.
    Check(RulesCheckArgs),

    /// List available rules
    List(RulesListArgs),
}

#[derive(Args, Debug)]
pub struct RulesCheckArgs {
    #[arg(long, short = 'W')]
    /// Treat warnings as errors
    pub warnings_as_errors: bool,

    #[arg(long)]
    /// Perform additional nit-picking checks
    pub pedantic: bool,

    #[command(flatten)]
    pub rules: RuleSpecifierArgs,
}

#[derive(Args, Debug)]
pub struct RulesListArgs {
    #[command(flatten)]
    pub rules: RuleSpecifierArgs,

    #[command(flatten)]
    pub output_args: OutputArgs<RulesListOutputFormat>,
}

// -----------------------------------------------------------------------------
// rules list output format
// -----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum RulesListOutputFormat {
    /// A text-based format designed for humans
    Human,

    /// Pretty-printed JSON format
    Json,
}

// -----------------------------------------------------------------------------
// `datastore` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct DatastoreArgs {
    #[command(subcommand)]
    pub command: DatastoreCommand,
}

#[derive(Subcommand, Debug)]
pub enum DatastoreCommand {
    /// Initialize a new datastore
    Init(DatastoreInitArgs),

    /// Export a datastore
    Export(DatastoreExportArgs),
}

#[derive(Args, Debug)]
pub struct DatastoreInitArgs {
    /// Initialize the datastore at specified path
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        env("NP_DATASTORE"),
        default_value=DEFAULT_DATASTORE,
    )]
    pub datastore: PathBuf,
}

#[derive(Args, Debug)]
pub struct DatastoreExportArgs {
    /// Datastore to export
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        env("NP_DATASTORE"),
        default_value=DEFAULT_DATASTORE,
    )]
    pub datastore: PathBuf,

    /// Write output to the specified path
    #[arg(long, short, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub output: PathBuf,

    /// Write output in the specified format
    #[arg(long, short, value_name = "FORMAT", default_value = "tgz")]
    pub format: DatastoreExportOutputFormat,
}

// -----------------------------------------------------------------------------
// datastore export output format
// -----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum DatastoreExportOutputFormat {
    /// gzipped tarball
    Tgz,
}

// -----------------------------------------------------------------------------
// `scan` command
// -----------------------------------------------------------------------------
/// Arguments for the `scan` command
#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Use the specified datastore
    ///
    /// The datastore will be created if it does not exist.
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        env("NP_DATASTORE"),
        default_value=DEFAULT_DATASTORE,
    )]
    pub datastore: PathBuf,

    /// Use N parallel scanning threads
    #[arg(long("jobs"), short('j'), value_name="N", default_value_t=default_scan_jobs())]
    pub num_jobs: usize,

    #[command(flatten)]
    pub rules: RuleSpecifierArgs,

    #[command(flatten)]
    pub input_specifier_args: InputSpecifierArgs,

    #[command(flatten)]
    pub content_filtering_args: ContentFilteringArgs,

    #[command(flatten)]
    pub metadata_args: MetadataArgs,

    /// Include up to the specified number of bytes before and after each match
    ///
    /// The default value typically gives between 4 and 7 lines of context before and after each
    /// match.
    #[arg(
        long,
        value_name = "BYTES",
        default_value_t = 256,
        help_heading = "Data Collection Options"
    )]
    pub snippet_length: usize,

    /// Specify which blobs will be copied in entirety to the datastore
    ///
    /// If this option is enabled, corresponding blobs will be written to the `blobs` directory within the datastore.
    /// The format of that directory is similar to Git's "loose" object format:
    /// the first 2 characters of the hex-encoded blob ID name a subdirectory, and the remaining characters are used as the filename.
    ///
    /// This mechanism exists to aid in ad-hoc downstream investigation.
    /// Copied blobs are not used elsewhere in Nosey Parker at this point.
    #[arg(
        long,
        default_value_t=CopyBlobsMode::None,
        value_name="MODE",
        help_heading="Data Collection Options",
    )]
    pub copy_blobs: CopyBlobsMode,

    /// Specify the format for blobs copied by the `--copy-blobs` option
    #[arg(long, value_name="FORMAT", default_value_t=DEFAULT_COPY_BLOBS_FORMAT)]
    pub copy_blobs_format: CopyBlobsFormat,
}

#[derive(Args, Debug)]
#[command(next_help_heading = "Rule Selection Options")]
pub struct RuleSpecifierArgs {
    /// Load additional rules and rulesets from the specified file or directory
    ///
    /// The paths can be either files or directories.
    /// Directories are recursively walked and all discovered YAML files of rules and rulesets will be loaded.
    ///
    /// This option can be repeated.

    #[arg(long, value_name = "PATH", value_hint = ValueHint::AnyPath)]
    pub rules_path: Vec<PathBuf>,

    /// Enable the ruleset with the specified ID
    ///
    /// The ID must resolve to a built-in ruleset or to an additional ruleset loaded with the
    /// `--rules=PATH` option.
    ///
    /// The special `all` ID causes all loaded rules to be used.
    ///
    /// This option can be repeated.
    ///
    /// Specifying this option disables the default ruleset.
    /// If you want to use a custom ruleset in addition to the default ruleset, specify this option twice, e.g., `--ruleset default --ruleset CUSTOM_ID`.
    #[arg(long, value_name = "ID", default_values_t=["default".to_string()])]
    pub ruleset: Vec<String>,

    /// Control whether built-in rules and rulesets are loaded.
    #[arg(long, default_value_t=true, action=ArgAction::Set, value_name="BOOL")]
    pub load_builtins: bool,
}

/// The mode to use for cloning a Git repository
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum GitCloneMode {
    /// Match the behavior of `git clone --bare`
    Bare,

    /// Match the behavior of `git clone --mirror`
    ///
    /// This will clone the most possible content.
    /// When cloning repositories hosted on GitHub, this mode may clone objects that come from forks.
    Mirror,
}

#[cfg(feature = "github")]
/// Which GitHub repositories to select
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum GitHubRepoType {
    /// Select both source repositories and fork repositories
    All,

    /// Only source repositories, i.e., ones that are not forks
    Source,

    /// Only fork repositories
    #[value(alias = "forks")]
    Fork,
}

#[cfg(feature = "github")]
impl From<GitHubRepoType> for noseyparker::github::RepoType {
    fn from(val: GitHubRepoType) -> Self {
        match val {
            GitHubRepoType::All => noseyparker::github::RepoType::All,
            GitHubRepoType::Source => noseyparker::github::RepoType::Source,
            GitHubRepoType::Fork => noseyparker::github::RepoType::Fork,
        }
    }
}

/// The method of handling history in discovered Git repositories
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum GitHistoryMode {
    /// Scan all history
    Full,

    // XXX: add an option to support bounded history, such as just blobs in the repo HEAD
    /// Scan no history
    None,
}

#[derive(Args, Debug)]
#[command(next_help_heading = "Metadata Collection Options")]
pub struct MetadataArgs {
    /// Specify which blobs will have metadata recorded
    #[arg(long, default_value_t=BlobMetadataMode::Matching, value_name="MODE")]
    pub blob_metadata: BlobMetadataMode,

    /// Specify which Git commit provenance metadata will be collected
    ///
    /// This should not need to be changed unless you are running into performance problems on a
    /// problematic Git repository input.
    #[arg(long, default_value_t=GitBlobProvenanceMode::FirstSeen, value_name="MODE")]
    pub git_blob_provenance: GitBlobProvenanceMode,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum BlobMetadataMode {
    /// Record metadata for all encountered blobs
    All,

    /// Record metadata only for blobs with matches
    Matching,

    /// Record metadata for no blobs
    None,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum CopyBlobsMode {
    /// Copy all encountered blobs
    All,

    /// Copy only blobs with matches
    Matching,

    /// Copy no blobs
    None,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum CopyBlobsFormat {
    #[cfg(feature = "parquet")]
    /// Parquet format
    Parquet,

    /// Plain files, similar to Git's loose object format
    Files,
}

#[cfg(feature = "parquet")]
const DEFAULT_COPY_BLOBS_FORMAT: CopyBlobsFormat = CopyBlobsFormat::Parquet;
#[cfg(not(feature = "parquet"))]
const DEFAULT_COPY_BLOBS_FORMAT: CopyBlobsFormat = CopyBlobsFormat::Files;

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum GitBlobProvenanceMode {
    /// The Git repository and set of commits and accompanying pathnames in which a blob is first
    /// seen
    FirstSeen,

    /// Only the Git repository in which a blob is seen
    Minimal,
}

#[derive(Args, Debug)]
#[command(next_help_heading = "Input Specifier Options")]
pub struct InputSpecifierArgs {
    #[cfg(feature = "github")]
    /// Scan the specified file, directory, or local Git repository
    #[arg(
        value_name="INPUT",
        value_hint=ValueHint::AnyPath,
        required_unless_present_any([
            "github_user",
            "github_organization",
            "git_url",
            "all_github_organizations",
            "enumerators",
        ]),
        display_order=1,
    )]
    pub path_inputs: Vec<PathBuf>,

    #[cfg(not(feature = "github"))]
    /// Scan the specified file, directory, or local Git repository
    #[arg(
        value_name="INPUT",
        value_hint=ValueHint::AnyPath,
        display_order=1,
    )]
    pub path_inputs: Vec<PathBuf>,

    /// Clone and scan the Git repository at the specified URL
    ///
    /// Only https URLs without credentials, query parameters, or fragment identifiers are supported.
    ///
    /// This option can be repeated.
    #[arg(
        long,
        value_name = "URL",
        value_hint = ValueHint::Url,
        display_order = 10,
    )]
    pub git_url: Vec<GitUrl>,

    /// Read inputs from a JSONL enumerator file (experimental)
    ///
    /// This can be used to stream inputs from other processes without having to write them to disk.
    /// Shell process substitution (e.g., `--enumerator=<(my-enumerator-program)`) can make this
    /// ergonomic.
    ///
    /// Each line of the enumerator file should be a JSON object with one of the following forms:
    ///
    ///     { "content_base64": "base64-encoded bytestring to scan", "provenance": <arbitrary object> }
    ///     { "content": "utf8 string to scan", "provenance": <arbitrary object> }
    ///
    /// This option can be repeated.
    #[arg(
        long("enumerator"),
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        display_order=15,
    )]
    pub enumerators: Vec<PathBuf>,

    #[cfg(feature = "github")]
    /// Clone and scan accessible repositories belonging to the specified GitHub user
    ///
    /// This option can be repeated.
    #[arg(long, value_name = "NAME", display_order = 20)]
    pub github_user: Vec<String>,

    #[cfg(feature = "github")]
    /// Clone and scan accessible repositories belonging to the specified GitHub organization
    ///
    /// This option can be repeated.
    #[arg(
        long,
        visible_alias = "github-org",
        value_name = "NAME",
        display_order = 20
    )]
    pub github_organization: Vec<String>,

    #[cfg(feature = "github")]
    /// Clone and scan accessible repositories from all accessible GitHub organizations
    ///
    /// This only works with a GitHub Enterprise Server instance.
    /// A non-default option for the `--github-api-url` option must be specified.
    #[arg(
        long,
        visible_alias = "all-github-orgs",
        requires = "github_api_url",
        display_order = 21
    )]
    pub all_github_organizations: bool,

    #[cfg(feature = "github")]
    /// Use the specified URL for GitHub API access
    ///
    /// If accessing a GitHub Enterprise Server instance, this value should be the entire base URL
    /// include the `api/v3` portion, e.g., `https://github.example.com/api/v3`.
    #[arg(
        long,
        visible_alias = "api-url",
        value_name = "URL",
        value_hint = ValueHint::Url,
        default_value = "https://api.github.com/",
        display_order = 30
    )]
    pub github_api_url: Url,

    #[cfg(feature = "github")]
    /// Clone and scan GitHub repos only of the given type
    #[arg(
        long,
        value_name = "TYPE",
        default_value_t = GitHubRepoType::Source,
    )]
    pub github_repo_type: GitHubRepoType,

    /// Use the specified method for cloning Git repositories
    #[arg(long, value_name = "MODE", display_order = 40, default_value_t=GitCloneMode::Bare, alias="git-clone-mode")]
    pub git_clone: GitCloneMode,

    /// Use the specified mode for handling Git history
    ///
    /// Git history can be completely ignored when scanning by using `--git-history=none`.
    /// Note that this will interfere with other input specifiers that cause Git repositories to be automatically cloned.
    /// For example, specifying an input with `--git-url=<URL>` while simultaneously using `--git-history=none` will not result in useful scanning.
    #[arg(long, value_name = "MODE", display_order = 50, default_value_t=GitHistoryMode::Full)]
    pub git_history: GitHistoryMode,
}

/// This struct represents options to control content discovery.
#[derive(Args, Debug)]
#[command(next_help_heading = "Content Filtering Options")]
pub struct ContentFilteringArgs {
    /// Do not scan files larger than the specified size
    ///
    /// The value is parsed as a floating point literal, and hence fractional values can be supplied.
    /// A non-positive value means "no limit".
    /// Note that scanning requires reading the entire contents of each file into memory, so using an excessively large limit may be problematic.
    #[arg(
        long("max-file-size"),
        default_value_t = 100.0,
        value_name = "MEGABYTES",
        allow_negative_numbers = true
    )]
    pub max_file_size_mb: f64,

    /// Use custom path-based ignore rules from the specified file
    ///
    /// The ignore file should contain gitignore-style rules.
    ///
    /// This option can be repeated.
    #[arg(long, short, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub ignore: Vec<PathBuf>,
    /*
    /// Do not scan files that appear to be binary
    #[arg(long)]
    pub skip_binary_files: bool,
    */
}

impl ContentFilteringArgs {
    pub fn max_file_size_bytes(&self) -> Option<u64> {
        if self.max_file_size_mb < 0.0 {
            None
        } else {
            Some((self.max_file_size_mb * 1024.0 * 1024.0) as u64)
        }
    }
}

// -----------------------------------------------------------------------------
// `summarize` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct SummarizeArgs {
    /// Use the specified datastore
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        env("NP_DATASTORE"),
        default_value=DEFAULT_DATASTORE,
    )]
    pub datastore: PathBuf,

    #[command(flatten)]
    pub output_args: OutputArgs<SummarizeOutputFormat>,
}

// -----------------------------------------------------------------------------
// `report` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct ReportArgs {
    /// Use the specified datastore
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        env("NP_DATASTORE"),
        default_value=DEFAULT_DATASTORE,
    )]
    pub datastore: PathBuf,

    #[command(flatten)]
    pub filter_args: ReportFilterArgs,

    #[command(flatten)]
    pub output_args: OutputArgs<ReportOutputFormat>,
}

#[derive(Args, Debug)]
#[command(next_help_heading = "Filtering Options")]
pub struct ReportFilterArgs {
    /// Limit the number of matches per finding to at most N
    ///
    /// A non-positive value means "no limit".
    #[arg(
        long,
        default_value_t = 3,
        value_name = "N",
        allow_negative_numbers = true
    )]
    pub max_matches: i64,

    /// Limit the number of provenance entries per match to at most N
    ///
    /// A non-positive value means "no limit".
    #[arg(
        long,
        default_value_t = 3,
        value_name = "N",
        allow_negative_numbers = true
    )]
    pub max_provenance: i64,

    /// Only report findings that have a mean score of at least N
    ///
    /// Scores are floating point numbers in the range [0, 1].
    /// Use the value `0` to disable this filtering.
    ///
    /// Findings that do not have a score computed will be included regardless of this setting.
    #[arg(long, default_value_t = 0.05, value_name = "SCORE")]
    pub min_score: f64,

    /// Include only findings with the assigned status
    #[arg(long, value_name = "STATUS")]
    pub finding_status: Option<FindingStatus>,

    /// Suppress redundant matches and findings
    ///
    /// A match is considered redundant to another if they overlap significantly within the same
    /// blob and satisfy a handful of heuristics.
    #[arg(long, default_value_t=true, action=ArgAction::Set, value_name="BOOL")]
    pub suppress_redundant: bool,
}

#[derive(ValueEnum, Debug, Display, Clone, Copy)]
#[clap(rename_all = "lower")]
#[strum(serialize_all = "lowercase")]
pub enum FindingStatus {
    /// Findings with `accept` matches
    Accept,
    /// Findings with `reject` matches
    Reject,
    /// Findings with both `accept` and `reject` matches
    Mixed,
    /// Findings without any `accept` or `reject` matches
    Null,
}

// -----------------------------------------------------------------------------
// `annotations` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct AnnotationsArgs {
    #[command(subcommand)]
    pub command: AnnotationsCommand,
}

#[derive(Subcommand, Debug)]
pub enum AnnotationsCommand {
    /// Export annotations from a datastore (experimental)
    Export(AnnotationsExportArgs),

    /// Import annotations into a datastore (experimental)
    Import(AnnotationsImportArgs),
}

#[derive(Args, Debug)]
pub struct AnnotationsExportArgs {
    /// Use the specified datastore
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        env("NP_DATASTORE"),
        default_value=DEFAULT_DATASTORE,
    )]
    pub datastore: PathBuf,

    /// Write annotations to the specified path
    ///
    /// If this argument is not provided, stdout will be used.
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct AnnotationsImportArgs {
    /// Use the specified datastore
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        env("NP_DATASTORE"),
        default_value=DEFAULT_DATASTORE,
    )]
    pub datastore: PathBuf,

    /// Read annotations from the specified path
    ///
    /// If this argument is not provided, stdin will be used.
    #[arg(
        long,
        short,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    pub input: Option<PathBuf>,
}

// -----------------------------------------------------------------------------
// `generate` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct GenerateArgs {
    #[command(subcommand)]
    pub command: GenerateCommand,
}

#[derive(Subcommand, Debug)]
pub enum GenerateCommand {
    /// Generate man pages
    #[command(name = "manpages")]
    ManPages(ManPagesArgs),

    /// Generate the JSON schema for the output of the `report` command
    JsonSchema(JsonSchemaArgs),

    /// Generate shell completions
    ShellCompletions(ShellCompletionsArgs),
}

// -----------------------------------------------------------------------------
// `generate shell-completions` command
// -----------------------------------------------------------------------------
#[derive(ValueEnum, Debug, Display, Clone)]
#[clap(rename_all = "lower")]
#[strum(serialize_all = "lowercase")]
pub enum ShellFormat {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

#[derive(Args, Debug)]
pub struct ShellCompletionsArgs {
    #[arg(long, short, value_name = "SHELL")]
    pub shell: ShellFormat,
}

// -----------------------------------------------------------------------------
// `generate json-schema` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct JsonSchemaArgs {
    /// Write output to the specified path
    ///
    /// If this argument is not provided, stdout will be used.
    #[arg(long, short, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,
}

// -----------------------------------------------------------------------------
// `generate manpages` command
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct ManPagesArgs {
    /// Write output to the specified directory
    #[arg(long, short, value_name = "PATH", value_hint = ValueHint::DirPath, default_value="manpages")]
    pub output: PathBuf,
}

// -----------------------------------------------------------------------------
// output options
// -----------------------------------------------------------------------------
#[derive(Args, Debug)]
#[command(next_help_heading = "Output Options")]
pub struct OutputArgs<Format: ValueEnum + Send + Sync + 'static> {
    /// Write output to the specified path
    ///
    /// If this argument is not provided, stdout will be used.
    #[arg(long, short, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Write output in the specified format
    // FIXME: make this optional, and if not specified, infer from the extension of the output file
    #[arg(long, short, value_name = "FORMAT", default_value = "human")]
    pub format: Format,
}

impl<Format: ValueEnum + Send + Sync> OutputArgs<Format> {
    /// Get a writer for the specified output destination.
    pub fn get_writer(&self) -> std::io::Result<Box<dyn std::io::Write>> {
        get_writer_for_file_or_stdout(self.output.as_ref())
    }
}

// -----------------------------------------------------------------------------
// report output format
// -----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum ReportOutputFormat {
    /// A text-based format designed for humans
    Human,

    /// Pretty-printed JSON format
    Json,

    /// JSON Lines format
    ///
    /// This is a sequence of JSON objects, one per line.
    Jsonl,

    /// SARIF format (experimental)
    ///
    /// This is the Static Analysis Results Interchange Format, a standardized JSON-based format used by many tools.
    /// See the spec at <https://docs.oasis-open.org/sarif/sarif/v2.1.0/cs01/sarif-v2.1.0-cs01.html>.
    ///
    /// Support for SARIF output is experimental.
    /// If you run into problems when using this, please create an issue in the GitHub project: <https://github.com/praetorian-inc/noseyparker>.
    Sarif,
}

// -----------------------------------------------------------------------------
// summarize output format
// -----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum SummarizeOutputFormat {
    /// A text-based format designed for humans
    Human,

    /// Pretty-printed JSON format
    Json,

    /// JSON Lines format
    ///
    /// This is a sequence of JSON objects, one per line.
    Jsonl,
}

#[cfg(feature = "github")]
// -----------------------------------------------------------------------------
// github output format
// -----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum GitHubOutputFormat {
    /// A text-based format designed for humans
    Human,

    /// Pretty-printed JSON format
    Json,

    /// JSON Lines format
    ///
    /// This is a sequence of JSON objects, one per line.
    Jsonl,
}
