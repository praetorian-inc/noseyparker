use assert_cmd::prelude::*;
// use assert_fs::prelude::*;
use insta::{assert_display_snapshot, assert_snapshot};
// use predicates::prelude::*;
use std::process::Command;

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

fn noseyparker() -> Command {
    Command::cargo_bin("noseyparker").expect("noseyparker should be executable")
}

#[test]
fn test_noseyparker_no_args() {
    assert_cmd_snapshot!(noseyparker().assert().failure());
}

#[test]
fn test_noseyparker_help() {
    assert_cmd_snapshot!(noseyparker().arg("help").assert().success());
}

#[test]
fn test_noseyparker_help_scan() {
    assert_cmd_snapshot!(noseyparker().args(&["help", "scan"]).assert().success());
}

#[test]
fn test_noseyparker_help_summarize() {
    assert_cmd_snapshot!(noseyparker().args(&["help", "summarize"]).assert().success());
}

#[test]
fn test_noseyparker_help_report() {
    assert_cmd_snapshot!(noseyparker().args(&["help", "report"]).assert().success());
}

#[test]
fn test_noseyparker_help_datastore() {
    assert_cmd_snapshot!(noseyparker().args(&["help", "datastore"]).assert().success());
}

#[test]
fn test_noseyparker_help_rules() {
    assert_cmd_snapshot!(noseyparker().args(&["help", "rules"]).assert().success());
}
