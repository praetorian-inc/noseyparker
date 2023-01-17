use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use assert_fs::{fixture::ChildPath, TempDir};
use insta::{assert_display_snapshot, assert_snapshot, with_settings};
// use predicates::prelude::*;
use std::path::Path;
use std::process::Command;

// -------------------------------------------------------------------------------------------------
// Utilities
// -------------------------------------------------------------------------------------------------

/// Use `insta` to do snapshot testing against a command's exit code, stdout, and stderr.
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

/// Build a `Command` for the `noseyparker` crate binary.
fn noseyparker() -> Command {
    Command::cargo_bin("noseyparker").expect("noseyparker should be executable")
}

fn is_match(pat: &str) -> predicates::str::RegexPredicate {
    predicates::str::is_match(pat).expect("pattern should compile")
}

pub struct ScanEnv {
    pub root: TempDir,
    pub datastore: ChildPath,
}

impl ScanEnv {
    pub fn new() -> Self {
        let root = TempDir::new().expect("should be able to create tempdir");
        let datastore = root.child("datastore");
        assert!(!datastore.exists());

        Self { root, datastore }
    }

    pub fn input_file(&self, name: &str) -> ChildPath {
        let input = self.root.child(name);
        input.touch().expect("should be able to write input file");
        assert!(input.is_file());
        input
    }

    pub fn input_dir(&self, name: &str) -> ChildPath {
        let input = self.root.child(name);
        input
            .create_dir_all()
            .expect("should be able to create input directory");
        assert!(input.is_dir());
        input
    }

    pub fn child(&self, name: &str) -> ChildPath {
        self.root.child(name)
    }

    pub fn dspath(&self) -> &Path {
        self.datastore.path()
    }
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[test]
fn test_noseyparker_no_args() {
    assert_cmd_snapshot!(noseyparker().assert().failure());
}

#[test]
fn test_noseyparker_help() {
    assert_cmd_snapshot!(noseyparker!("help").assert().success());
}

#[test]
fn test_noseyparker_help_scan() {
    with_settings!({
        filters => vec![
            (r"(?m)(scanning jobs\s+)\[default: \d+\]", r"$1[default: DEFAULT]")
        ],
    }, {
        assert_cmd_snapshot!(noseyparker!("help", "scan").assert().success());
    });
}

#[test]
fn test_noseyparker_help_summarize() {
    assert_cmd_snapshot!(noseyparker!("help", "summarize").assert().success());
}

#[test]
fn test_noseyparker_help_report() {
    assert_cmd_snapshot!(noseyparker!("help", "report").assert().success());
}

#[test]
fn test_noseyparker_help_datastore() {
    assert_cmd_snapshot!(noseyparker!("help", "datastore").assert().success());
}

#[test]
fn test_noseyparker_help_rules() {
    assert_cmd_snapshot!(noseyparker!("help", "rules").assert().success());
}

#[test]
fn test_noseyparker_scan_emptydir() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker!("scan", "--datastore", scan_env.dspath(), input.path())
        .assert()
        .success()
        .stdout(is_match(r#"^Scanned 0B from 0 blobs in .*; 0/0 new matches"#));
}

#[test]
fn test_noseyparker_scan_datastore_argorder() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker!("scan", input.path(), "--datastore", scan_env.dspath())
        .assert()
        .success()
        .stdout(is_match(r#"^Scanned 0B from 0 blobs in .*; 0/0 new matches"#));
}

#[test]
fn test_noseyparker_scan_datastore_short() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker!("scan", "-d", scan_env.dspath(), input.path())
        .assert()
        .success()
        .stdout(is_match(r#"^Scanned 0B from 0 blobs in .*; 0/0 new matches"#));
}

#[test]
fn test_noseyparker_scan_datastore_envvar() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker!("scan", input.path())
        .env("NP_DATASTORE", scan_env.dspath())
        .assert()
        .success()
        .stdout(is_match(r#"^Scanned 0B from 0 blobs in .*; 0/0 new matches"#));
}

#[test]
fn test_noseyparker_scan_emptyfile() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file("empty_file");
    noseyparker!("scan", "--datastore", scan_env.dspath(), input.path())
        .assert()
        .success()
        .stdout(is_match(r#"^Scanned 0B from 1 blobs in .*; 0/0 new matches"#));
}

#[test]
fn test_noseyparker_scan_emptyfiles() {
    let scan_env = ScanEnv::new();
    let input1 = scan_env.input_file("empty_file1");
    let input2 = scan_env.input_file("empty_file2");
    noseyparker!("scan", "--datastore", scan_env.dspath(), input1.path(), input2.path())
        .assert()
        .success()
        .stdout(is_match(r#"^Scanned 0B from 2 blobs in .*; 0/0 new matches"#));
}
