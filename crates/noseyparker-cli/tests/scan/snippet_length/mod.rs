use super::*;

#[test]
fn scan_invalid_snippet_length() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_failure!(
        "scan",
        "--datastore",
        scan_env.dspath(),
        input.path(),
        "--snippet-length=-1"
    )
    .stderr(is_match("error: invalid value .* for .*: invalid digit found in string"));
}

#[test]
fn scan_changing_snippet_length() {
    let scan_env = ScanEnv::new();
    let input = scan_env.large_input_file_with_secret("input.txt");

    // first scan with non-default short snippet length
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path(), "--snippet-length=16")
        .stdout(match_scan_stats("1.41 KiB", 1, 1, 1));

    assert_cmd_snapshot!(noseyparker_success!("summarize", "-d", scan_env.dspath()));

    with_settings!({
        filters => get_report_stdout_filters(),
    }, {
        assert_cmd_snapshot!(noseyparker_success!("report", "-d", scan_env.dspath()));
    });

    let cmd = noseyparker_success!("report", "-d", scan_env.dspath(), "--format=json");
    let json_output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
    with_settings!({
        redactions => get_report_json_redactions(),
    }, {
        assert_json_snapshot!(json_output);
    });

    // now rescan with longer snippet length
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path(), "--snippet-length=32")
        .stdout(match_scan_stats("1.41 KiB", 1, 0, 1));

    assert_cmd_snapshot!(noseyparker_success!("summarize", "-d", scan_env.dspath()));

    with_settings!({
        filters => get_report_stdout_filters(),
    }, {
        assert_cmd_snapshot!(noseyparker_success!("report", "-d", scan_env.dspath()));
    });

    let cmd = noseyparker_success!("report", "-d", scan_env.dspath(), "--format=json");
    let json_output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
    with_settings!({
        redactions => get_report_json_redactions(),
    }, {
        assert_json_snapshot!(json_output);
    });
}
