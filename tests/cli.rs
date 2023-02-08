use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use assert_fs::{fixture::ChildPath, TempDir};
use indoc::indoc;
use insta::{assert_display_snapshot, assert_snapshot, assert_json_snapshot, with_settings};
use lazy_static::lazy_static;
// use predicates::prelude::*;
use predicates::str::RegexPredicate;
use std::path::Path;
use std::process::Command;

// -------------------------------------------------------------------------------------------------
// Utilities
// -------------------------------------------------------------------------------------------------

/// Use `insta` to do snapshot testing against a command's exit code, stdout, and stderr.
///
/// The given expression should be an `assert_cmd::assert::Assert`.
macro_rules! assert_cmd_snapshot {
    ( $cmd:expr ) => {
        let cmd = $cmd;
        let output = cmd.get_output();
        let status = output.status;
        assert_display_snapshot!(status);
        let stdout = String::from_utf8(output.stdout.clone()).unwrap();
        assert_snapshot!(stdout);
        let stderr = String::from_utf8(output.stderr.clone()).unwrap();
        assert_snapshot!(stderr);
    };
}

/// Build a `Command` for the `noseyparker` crate binary with variadic command-line arguments.
///
/// The arguments can be anything that is allowed by `Command::arg`.
macro_rules! noseyparker {
    ( $( $arg:expr ),* ) => {
        {
            let mut cmd = noseyparker();
            $(
                cmd.arg($arg);
            )*
            cmd
        }
    }
}

/// Build an `assert_cmd::assert::Assert` by calling `noseyparker!(args).assert().success()`.
macro_rules! noseyparker_success {
    ( $( $arg:expr ),* ) => { noseyparker!($( $arg ),*).assert().success() }
}

/// Build an `assert_cmd::assert::Assert` by calling `noseyparker!(args).assert().failure()`.
macro_rules! noseyparker_failure {
    ( $( $arg:expr ),* ) => { noseyparker!($( $arg ),*).assert().failure() }
}

lazy_static! {
    static ref NOSEYPARKER: escargot::CargoRun = escargot::CargoBuild::new()
        .bin("noseyparker")
        .run()
        .expect("noseyparker should be available");

    // We could use this to write tests for specific feature configurations:
    /*
    static ref NOSEYPARKER_RULE_PROFILING: escargot::CargoRun = escargot::CargoBuild::new()
        .bin("noseyparker")
        .no_default_features()
        .features("rule_profiling")
        .run()
        .expect("noseyparker with rule_profiling should be available");
    */
}

/// Build a `Command` for the `noseyparker` crate binary.
pub fn noseyparker() -> Command {
    // Command::cargo_bin("noseyparker").expect("noseyparker should be executable")
    NOSEYPARKER.command()
}

/// Create a `RegexPredicate` from the given pattern.
pub fn is_match(pat: &str) -> RegexPredicate {
    predicates::str::is_match(pat).expect("pattern should compile")
}

/// Create a `RegexPredicate` for matching a scan stats output message from Nosey Parker.
pub fn match_scan_stats(
    num_bytes: &str,
    num_blobs: u64,
    new_matches: u64,
    total_matches: u64,
) -> RegexPredicate {
    is_match(&format!(
        r"(?m)^Scanned {} from {} blobs in .*; {}/{} new matches$",
        num_bytes, num_blobs, new_matches, total_matches
    ))
}

/// Create a `RegexPredicate` for matching a "nothing was scanned" scan stats output message from
/// Nosey Parker.
pub fn match_nothing_scanned() -> RegexPredicate {
    match_scan_stats("0B", 0, 0, 0)
}

/// A type to represent a mock scanning environment for testing Nosey Parker.
pub struct ScanEnv {
    pub root: TempDir,
    pub datastore: ChildPath,
}

impl ScanEnv {
    /// Create a new mock scanning environment.
    pub fn new() -> Self {
        let root = TempDir::new().expect("should be able to create tempdir");
        let datastore = root.child("datastore");
        assert!(!datastore.exists());

        Self { root, datastore }
    }

    /// Create an empty file within this mock scanning environment with the given name.
    pub fn input_file(&self, name: &str) -> ChildPath {
        let input = self.root.child(name);
        input.touch().expect("should be able to write input file");
        assert!(input.is_file());
        input
    }

    /// Create an input file within this mock scanning environment with the given name.
    /// The created input file will have content containing a fake AWS key that should be detected.
    pub fn input_file_with_secret(&self, name: &str) -> ChildPath {
        let input = self.root.child(name);
        input.touch().expect("should be able to write input file");
        assert!(input.is_file());
        let contents = indoc! {r#"
            # This is fake configuration data
            USERNAME=the_dude
            AWS_KEY=AKIADEADBEEFDEADBEEF
        "#};
        input
            .write_str(contents)
            .expect("should be able to write input file contents");
        input
    }

    /// Create an empty directory within this mock scanning environment with the given name.
    pub fn input_dir(&self, name: &str) -> ChildPath {
        let input = self.root.child(name);
        input
            .create_dir_all()
            .expect("should be able to create input directory");
        assert!(input.is_dir());
        input
    }

    /// Create a name for a child entry within this mock scanning environment.
    ///
    /// The filesystem is not touched by this function; this merely produces a `ChildPath`.
    pub fn child(&self, name: &str) -> ChildPath {
        self.root.child(name)
    }

    /// Get the path to the Nosey Parker datastore directory within this mock scanning environment.
    pub fn dspath(&self) -> &Path {
        self.datastore.path()
    }
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[test]
fn test_noseyparker_no_args() {
    assert_cmd_snapshot!(noseyparker!().assert().failure());
}

#[test]
fn test_noseyparker_help() {
    assert_cmd_snapshot!(noseyparker_success!("help"));
}

#[test]
fn test_noseyparker_help_scan() {
    with_settings!({
        filters => vec![
            (r"(?m)(scanning jobs\s+)\[default: \d+\]", r"$1[default: DEFAULT]")
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("help", "scan"));
    });
}

#[test]
fn test_noseyparker_help_summarize() {
    assert_cmd_snapshot!(noseyparker_success!("help", "summarize"));
}

#[test]
fn test_noseyparker_help_report() {
    assert_cmd_snapshot!(noseyparker_success!("help", "report"));
}

#[test]
fn test_noseyparker_help_datastore() {
    assert_cmd_snapshot!(noseyparker_success!("help", "datastore"));
}

#[test]
fn test_noseyparker_help_rules() {
    assert_cmd_snapshot!(noseyparker_success!("help", "rules"));
}

#[test]
fn test_noseyparker_help_github() {
    assert_cmd_snapshot!(noseyparker_success!("help", "github"));
}

#[test]
fn test_noseyparker_help_github_repos() {
    assert_cmd_snapshot!(noseyparker_success!("help", "github", "repos"));
}

#[test]
fn test_noseyparker_github_repos_list_noargs() {
    assert_cmd_snapshot!(noseyparker_failure!("github", "repos", "list"));
}

#[test]
fn test_noseyparker_github_repos_list_org_badtoken() {
    let cmd = noseyparker()
        .args(&["github", "repos", "list", "--org", "praetorian-inc"])
        .env("GITHUB_TOKEN", "hahabogus")
        .assert()
        .failure();
    assert_cmd_snapshot!(cmd);
}

#[test]
fn test_noseyparker_github_repos_list_user_badtoken() {
    let cmd = noseyparker()
        .args(&["github", "repos", "list", "--user", "octocat"])
        .env("GITHUB_TOKEN", "hahabogus")
        .assert()
        .failure();
    assert_cmd_snapshot!(cmd);
}

// XXX These are disabled for now due to GitHub Actions rate limiting troubles.
// FIXME: Figure out how to plumb an access token to these tests, then re-enable them.
/*
#[test]
fn test_noseyparker_github_repos_list_user_unauthenticated_human_format() {
    noseyparker_success!("github", "repos", "list", "--user", "octocat")
        .stdout(predicates::str::contains("https://github.com/octocat/Spoon-Knife.git"))
        .stderr(predicates::str::is_empty());
}

#[test]
fn test_noseyparker_github_repos_list_user_unauthenticated_jsonl_format() {
    noseyparker_success!("github", "repos", "list", "--user", "octocat", "--format", "jsonl")
        .stdout(predicates::str::contains("\"https://github.com/octocat/Spoon-Knife.git\"\n"))
        .stderr(predicates::str::is_empty());
}

#[test]
fn test_noseyparker_github_repos_list_user_unauthenticated_json_format() {
    let cmd = noseyparker_success!("github", "repos", "list", "--user", "octocat", "--format", "json")
        .stderr(predicates::str::is_empty());
    let output = &cmd.get_output().stdout;
    let json_parsed: Vec<String> = serde_json::from_slice(output).expect("output should be well-formed JSON");
    assert!(json_parsed.contains(&String::from("https://github.com/octocat/Spoon-Knife.git")),
        "JSON output does not contain https://github.com/octocat/Spoon-Knife.git: {json_parsed:?}");
}
*/

// XXX additional `github repo list` cases to test:
//
// - multiple users / orgs
// - duplicated users / orgs: output list should be deduped
// - using a real access token

#[test]
fn test_noseyparker_scan_emptydir() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input.path())
        .stdout(match_nothing_scanned());
}

#[test]
fn test_noseyparker_scan_datastore_argorder() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_success!("scan", input.path(), "--datastore", scan_env.dspath())
        .stdout(match_nothing_scanned());
}

#[test]
fn test_noseyparker_scan_datastore_short() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_nothing_scanned());
}

#[test]
fn test_noseyparker_scan_datastore_envvar() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker!("scan", input.path())
        .env("NP_DATASTORE", scan_env.dspath())
        .assert()
        .success()
        .stdout(match_nothing_scanned());
}

#[test]
fn test_noseyparker_scan_emptyfile() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file("empty_file");
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("0B", 1, 0, 0));
}

#[test]
fn test_noseyparker_scan_emptyfiles() {
    let scan_env = ScanEnv::new();
    let input1 = scan_env.input_file("empty_file1");
    let input2 = scan_env.input_file("empty_file2");
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input1.path(), input2.path())
        .stdout(match_scan_stats("0B", 2, 0, 0));
}

#[test]
fn test_noseyparker_scan_file_symlink() {
    let scan_env = ScanEnv::new();
    let empty_file = scan_env.input_file("empty_file");
    let input = scan_env.child("empty_file_link");
    input.symlink_to_file(empty_file).unwrap();
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input.path())
        .stdout(match_nothing_scanned());
}

#[test]
fn test_noseyparker_scan_file_maxsize() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file("bigfile.dat");
    input.write_binary(&[b'a'; 1024 * 1024 * 10]).unwrap();

    // By default the input file gets scanned
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("10.00 MiB", 1, 0, 0));

    // With a restricted max file size, the file is not scanned
    noseyparker_success!(
        "scan",
        "--datastore",
        scan_env.dspath(),
        input.path(),
        "--max-file-size",
        "5"
    )
    .stdout(match_nothing_scanned());

    // Also check for alternatively-spelled versions of a couple arguments
    noseyparker_success!(
        "scan",
        format!("-d={}", scan_env.dspath().display()),
        "--max-file-size=5.00",
        input.path()
    )
    .stdout(match_nothing_scanned());
}

#[cfg(unix)]
#[test]
fn test_noseyparker_scan_unreadable_file() {
    use std::fs::{File, Permissions};
    use std::os::unix::fs::PermissionsExt;

    let scan_env = ScanEnv::new();
    let input = scan_env.input_file_with_secret("input.txt");
    // n.b. file value explicitly unnamed so it gets dropped
    File::open(input.path())
        .unwrap()
        .set_permissions(Permissions::from_mode(0o000))
        .unwrap();
    assert!(std::fs::read_to_string(input.path()).is_err());

    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(is_match("ERROR.*: Failed to load blob from .*: Permission denied"))
        .stdout(match_nothing_scanned());
}

#[test]
fn test_noseyparker_scan_secrets1() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file_with_secret("input.txt");

    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("81B", 1, 1, 1));

    assert_cmd_snapshot!(noseyparker_success!("summarize", "-d", scan_env.dspath()));

    with_settings!({
        filters => vec![
            (r"(?m)^(\s*File: ).*$", r"$1 <FILENAME>")
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("report", "-d", scan_env.dspath()));
    });


    let cmd = noseyparker_success!("report", "-d", scan_env.dspath(), "--format=json");
    let json_output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
    assert_json_snapshot!(json_output, {
        "[].matches[].provenance.path" => "<ROOT>/input.txt"
    });
}
