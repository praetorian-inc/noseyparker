//! Tests for Nosey Parker `help` functionality

use super::*;

#[test]
fn no_args() {
    assert_cmd_snapshot!(noseyparker!().assert().failure());
}

#[test]
fn help() {
    assert_cmd_snapshot!(noseyparker_success!("help"));
}

#[test]
fn help_short() {
    assert_cmd_snapshot!(noseyparker_success!("-h"));
}

#[test]
fn help_scan() {
    with_settings!({
        filters => vec![
            (r"(?m)(scanning threads\s+)\[default: \d+\]", r"$1[default: DEFAULT]"),
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("help", "scan"));
    });
}

#[test]
fn help_scan_short() {
    with_settings!({
        filters => vec![
            (r"(?m)(scanning threads\s+)\[default: \d+\]", r"$1[default: DEFAULT]"),
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("scan", "-h"));
    });
}

#[test]
fn help_summarize() {
    assert_cmd_snapshot!(noseyparker_success!("help", "summarize"));
}

#[test]
fn help_summarize_short() {
    assert_cmd_snapshot!(noseyparker_success!("summarize", "-h"));
}

#[test]
fn help_report() {
    with_settings!({
        filters => vec![
            (r"(?m)(denoising threads when using the CPU\s+)\[default: \d+\]", r"$1[default: DEFAULT]"),
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("help", "report"));
    });
}

#[test]
fn help_report_short() {
    with_settings!({
        filters => vec![
            (r"(?m)(denoising threads when using the CPU\s+)\[default: \d+\]", r"$1[default: DEFAULT]"),
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("report", "-h"));
    });
}

#[test]
fn help_datastore() {
    assert_cmd_snapshot!(noseyparker_success!("help", "datastore"));
}

#[test]
fn help_rules() {
    assert_cmd_snapshot!(noseyparker_success!("help", "rules"));
}

#[test]
fn help_github() {
    assert_cmd_snapshot!(noseyparker_success!("help", "github"));
}

#[test]
fn help_github_short() {
    assert_cmd_snapshot!(noseyparker_success!("github", "-h"));
}

#[test]
fn help_github_repos() {
    assert_cmd_snapshot!(noseyparker_success!("help", "github", "repos"));
}

#[test]
fn help_github_repos_short() {
    assert_cmd_snapshot!(noseyparker_success!("github", "repos", "-h"));
}

// #[test]
// fn version_short() {
//     assert_cmd_snapshot!(noseyparker_success!("-V"));
// }

#[test]
fn version_long() {
    with_settings!({
        filters => vec![
            (r"(?m)^(    [^:]+:[ \t]+).*$", r"$1<PLACEHOLDER>")
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("--version"));
    });
}

#[test]
fn version_command() {
    // there is no `version` command
    assert_cmd_snapshot!(noseyparker_failure!("version"));
}
