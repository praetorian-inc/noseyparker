//! Integration Test Utilities and Common Code

#![allow(dead_code)]

use indoc::indoc;
// use lazy_static::lazy_static;

pub use assert_cmd::prelude::*;
pub use assert_fs::prelude::*;
pub use assert_fs::{fixture::ChildPath, TempDir};
pub use insta::{
    assert_display_snapshot, assert_json_snapshot, assert_snapshot, internals::Redaction,
    with_settings,
};
pub use predicates::str::{is_empty, RegexPredicate};
pub use std::path::Path;
pub use std::process::Command;

/// Use `insta` to do snapshot testing against a command's exit code, stdout, and stderr.
///
/// The given expression should be an `assert_cmd::assert::Assert`.
#[macro_export]
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
#[macro_export]
macro_rules! noseyparker {
    ( $( $arg:expr ),* ) => {
        {
            let mut cmd = noseyparker_cmd();
            $(
                cmd.arg($arg);
            )*
            cmd
        }
    }
}

/// Build an `assert_cmd::assert::Assert` by calling `noseyparker!(args).assert().success()`.
#[macro_export]
macro_rules! noseyparker_success {
    ( $( $arg:expr ),* ) => { noseyparker!($( $arg ),*).assert().success() }
}

/// Build an `assert_cmd::assert::Assert` by calling `noseyparker!(args).assert().failure()`.
#[macro_export]
macro_rules! noseyparker_failure {
    ( $( $arg:expr ),* ) => { noseyparker!($( $arg ),*).assert().failure() }
}

/*
lazy_static! {
    // We could use escargot for running Cargo-built binaries.
    // But it seems to cause the entire project to be rebuilt once at test time!
    static ref NOSEYPARKER: escargot::CargoRun = escargot::CargoBuild::new()
        .current_release()
        .current_target()
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
pub fn noseyparker_cmd() -> Command {
    NOSEYPARKER.command()
}
*/

/// Get the command for the Nosey Parker binary under test.
///
/// By default, this is the binary defined in this crate.
/// However, if the `NP_TEST_PROGRAM` environment variable is set, its value is used instead.
/// Its value should be an absolute path to the desired `noseyparker` program to test.
///
/// This environment variable makes it possible to run the test suite on different versions of
/// Nosey Parker, such as a final release build or a Docker image.
/// For example:
///
///     NP_TEST_PROGRAM="$PWD"/release/bin/noseyparker cargo test --test test_noseyparker
///
pub fn noseyparker_cmd() -> Command {
    if let Ok(np) = std::env::var("NP_TEST_PROGRAM") {
        Command::new(np)
    } else {
        Command::cargo_bin("noseyparker-cli").expect("noseyparker should be executable")
    }
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
    match_scan_stats("0 B", 0, 0, 0)
}

/// A type to represent a mock scanning environment for testing Nosey Parker.
///
/// A mock scanning environment automatically chooses a directory name that can be used as a
/// datastore, and provides operations to create mock input files.
pub struct ScanEnv {
    pub root: TempDir,
    pub datastore: ChildPath,
}

impl ScanEnv {
    /// Create a new mock scanning environment.
    pub fn new() -> Self {
        // FIXME: need to be able to override the root directory to test Docker containers via `NP_TEST_PROGRAM`
        let root = TempDir::new().expect("should be able to create tempdir");
        assert!(root.exists());
        assert!(root.is_dir());
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

    /// Create a file within this mock scanning environment with the given name and contents.
    pub fn input_file_with_contents(&self, name: &str, contents: &str) -> ChildPath {
        let input = self.root.child(name);
        input.touch().expect("should be able to write input file");
        assert!(input.is_file());
        input
            .write_str(contents)
            .expect("should be able to write input file contents");
        input
    }

    /// Create a small input file within this mock scanning environment with the given name.
    /// The created input file will have content containing a fake GitHub PAT that should be detected.
    pub fn input_file_with_secret(&self, name: &str) -> ChildPath {
        self.input_file_with_contents(
            name,
            indoc! {r#"
            # This is fake configuration data
            USERNAME=the_dude
            GITHUB_KEY=ghp_XIxB7KMNdAr3zqWtQqhE94qglHqOzn1D1stg
        "#},
        )
    }

    /// Create a larger input file within this mock scanning environment with the given name.
    /// The created input file will have content containing a fake AWS key that should be detected.
    pub fn large_input_file_with_secret(&self, name: &str) -> ChildPath {
        self.input_file_with_contents(
            name,
            indoc! {r#"
            function lorem(ipsum, dolor = 1) {
              const sit = ipsum == null ? 0 : ipsum.sit;
              dolor = sit - amet(dolor);
              return sit ? consectetur(ipsum, 0, dolor < 0 ? 0 : dolor) : [];
            }
            function adipiscing(...elit) {
              if (!elit.sit) {
                return [];
              }
              const sed = elit[0];
              return eiusmod.tempor(sed) ? sed : [sed];
            }
            function incididunt(ipsum, ut = 1) {
              ut = labore.et(amet(ut), 0);
              const sit = ipsum == null ? 0 : ipsum.sit;
              if (!sit || ut < 1) {
                return [];
              }
              let dolore = 0;
              let magna = 0;
              const aliqua = new eiusmod(labore.ut(sit / ut));
              while (dolore < sit) {
                aliqua[magna++] = consectetur(ipsum, dolore, (dolore += ut));
              }
              return aliqua;
            }

            # This is fake configuration data
            USERNAME=the_dude
            GITHUB_KEY=ghp_XIxB7KMNdAr3zqWtQqhE94qglHqOzn1D1stg

            function lorem(ipsum, dolor = 1) {
              const sit = ipsum == null ? 0 : ipsum.sit;
              dolor = sit - amet(dolor);
              return sit ? consectetur(ipsum, 0, dolor < 0 ? 0 : dolor) : [];
            }
            function adipiscing(...elit) {
              if (!elit.sit) {
                return [];
              }
              const sed = elit[0];
              return eiusmod.tempor(sed) ? sed : [sed];
            }
            function incididunt(ipsum, ut = 1) {
              ut = labore.et(amet(ut), 0);
              const sit = ipsum == null ? 0 : ipsum.sit;
              if (!sit || ut < 1) {
                return [];
              }
              let dolore = 0;
              let magna = 0;
              const aliqua = new eiusmod(labore.ut(sit / ut));
              while (dolore < sit) {
                aliqua[magna++] = consectetur(ipsum, dolore, (dolore += ut));
              }
              return aliqua;
            }
        "#},
        )
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

/// Create an empty Git repo on the filesystem at `destination`.
pub fn create_empty_git_repo(destination: &Path) {
    Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(destination)
        .assert()
        .success()
        .stdout(is_empty())
        .stderr(is_empty());
}

pub fn get_report_stdout_filters() -> Vec<(&'static str, &'static str)> {
    vec![
        (r"(?m)^(\s*File: ).*$", r"$1 <FILENAME>"),
        (r"(?m)^(\s*Blob: ).*$", r"$1 <BLOB>"),
        (r"(?m)^(\s*Git repo: ).*$", r"$1 <REPO>"),
    ]
}

pub fn get_report_json_redactions() -> Vec<(&'static str, Redaction)> {
    vec![
        ("[].matches[].provenance[].path", Redaction::from("<ROOT>/input.txt")),
        ("[].matches[].provenance[].repo_path", Redaction::from("<REPO>")),
        ("[].score", insta::rounded_redaction(3)),
        ("[].matches[].score", insta::rounded_redaction(3)),
    ]
}
