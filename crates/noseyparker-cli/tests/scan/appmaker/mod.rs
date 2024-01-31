/// This module contains some more realistic integration tests.
/// Specifically, instead of using tiny synthetic input data, these use [Mozilla
/// Appmaker](https://github.com/mozilla-appmaker/appmaker), a long-defunct app-building project.
///
/// NOTE: we do not control the mozilla-appmaker GitHub organization or repository. However, it has
/// been archived for years. This is a potential source of test nondeterminacy, as that data could
/// conceivably be changed.
use super::*;
pub use pretty_assertions::assert_ne;

fn read_json(fname: &Path) -> anyhow::Result<serde_json::Value> {
    let file = std::fs::File::open(fname)?;
    let value = serde_json::from_reader(file)?;
    Ok(value)
}

/// This test runs a basic scanning workflow (scan -> summarize -> report) against the appmaker
/// repository, merely checking that its outputs don't change.
#[test]
fn scan_workflow_from_git_url() {
    let scan_env = ScanEnv::new();

    let datastore_arg = &format!("--datastore={}", scan_env.dspath().display());

    noseyparker_success!(
        "scan",
        datastore_arg,
        "--git-url=https://github.com/mozilla-appmaker/appmaker",
        "--ruleset=all"
    )
    .stdout(is_match(r"(?m)^Scanned 550.05 MiB from 7,928 blobs in .*; 23/23 new matches$"));

    assert_cmd_snapshot!(noseyparker_success!("summarize", datastore_arg));

    let report_json = scan_env.child("findings.json");
    noseyparker_success!("report", datastore_arg, "--format=json", "-o", report_json.path());
    with_settings!({
        redactions => get_report_json_redactions()
    }, {
        assert_json_snapshot!(read_json(report_json.path()).unwrap());
    });

    let report_txt = scan_env.child("findings.txt");
    noseyparker_success!("report", datastore_arg, "-o", report_txt.path());
    with_settings!({
        filters => get_report_stdout_filters()
    }, {
        assert_snapshot!(std::fs::read_to_string(report_txt.path()).unwrap());
    });

    // XXX Checking SARIF output format disabled for now until it's more actively supported
    // let report_sarif = scan_env.child("findings.sarif");
    // noseyparker_success!("report", datastore_arg, "--format=sarif", "-o", report_sarif.path());
    // assert_json_snapshot!(read_json(report_sarif.path()).unwrap());
}
