//! Tests for Nosey Parker `help` functionality

use super::*;

#[cfg(feature = "github")]
#[test]
fn no_args() {
    assert_cmd_snapshot!(noseyparker!().assert().failure());
}

#[cfg(not(feature = "github"))]
#[test]
fn no_args_nogithub() {
    assert_cmd_snapshot!(noseyparker!().assert().failure());
}

#[cfg(feature = "github")]
#[test]
fn help() {
    assert_cmd_snapshot!(noseyparker_success!("help"));
}

#[cfg(not(feature = "github"))]
#[test]
fn help_nogithub() {
    assert_cmd_snapshot!(noseyparker_success!("help"));
}

#[cfg(feature = "github")]
#[test]
fn help_short() {
    assert_cmd_snapshot!(noseyparker_success!("-h"));
}

#[cfg(not(feature = "github"))]
#[test]
fn help_short_nogithub() {
    assert_cmd_snapshot!(noseyparker_success!("-h"));
}

#[cfg(feature = "github")]
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

#[cfg(not(feature = "github"))]
#[test]
fn help_scan_nogithub() {
    with_settings!({
        filters => vec![
            (r"(?m)(scanning threads\s+)\[default: \d+\]", r"$1[default: DEFAULT]"),
        ],
    }, {
        assert_cmd_snapshot!(noseyparker_success!("help", "scan"));
    });
}

#[cfg(feature = "github")]
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

#[cfg(not(feature = "github"))]
#[test]
fn help_scan_short_nogithub() {
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

#[cfg(feature = "github")]
#[test]
fn help_github() {
    assert_cmd_snapshot!(noseyparker_success!("help", "github"));
}

#[cfg(feature = "github")]
#[test]
fn help_github_short() {
    assert_cmd_snapshot!(noseyparker_success!("github", "-h"));
}

#[cfg(feature = "github")]
#[test]
fn help_github_repos() {
    assert_cmd_snapshot!(noseyparker_success!("help", "github", "repos"));
}

#[cfg(feature = "github")]
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
