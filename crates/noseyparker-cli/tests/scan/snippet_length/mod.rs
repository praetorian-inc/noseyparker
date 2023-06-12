use super::*;

#[test]
fn scan_invalid_snippet_length() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_failure!("scan", "--datastore", scan_env.dspath(), input.path(), "--snippet-length=-1")
        .stderr(is_match("error: invalid value .* for .*: invalid digit found in string"));
}

#[test]
fn scan_changing_snippet_length() {
    let scan_env = ScanEnv::new();
    let input = scan_env.large_input_file_with_secret("input.txt");

    // first scan with non-default short snippet length
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path(), "--snippet-length=16")
        .stdout(match_scan_stats("1.39 KiB", 1, 1, 1));

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


    // now rescan with longer snippet length
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path(), "--snippet-length=32")
        .stdout(match_scan_stats("1.39 KiB", 1, 1, 1));

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
