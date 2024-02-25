use super::*;
pub use pretty_assertions::assert_ne;

#[test]
fn scan_emptydir() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input.path())
        .stdout(match_nothing_scanned());
}

#[test]
fn scan_datastore_argorder() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_success!("scan", input.path(), "--datastore", scan_env.dspath())
        .stdout(match_nothing_scanned());
}

#[test]
fn scan_datastore_short() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_nothing_scanned());
}

#[test]
fn scan_datastore_envvar() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_dir("empty_dir");
    noseyparker!("scan", input.path())
        .env("NP_DATASTORE", scan_env.dspath())
        .assert()
        .success()
        .stdout(match_nothing_scanned());
}

#[test]
fn scan_emptyfile() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file("empty_file");
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("0 B", 1, 0, 0));
}

#[test]
fn scan_emptyfiles() {
    let scan_env = ScanEnv::new();
    let input1 = scan_env.input_file("empty_file1");
    let input2 = scan_env.input_file("empty_file2");
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input1.path(), input2.path())
        .stdout(match_scan_stats("0 B", 2, 0, 0));
}

#[test]
fn scan_file_symlink() {
    let scan_env = ScanEnv::new();
    let empty_file = scan_env.input_file("empty_file");
    let input = scan_env.child("empty_file_link");
    input.symlink_to_file(empty_file).unwrap();
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input.path())
        .stdout(match_nothing_scanned());
}

#[test]
fn scan_file_maxsize() {
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

// FIXME: this one fails if you are running as root
#[cfg(unix)]
#[test]
fn scan_unreadable_file() {
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
        .stderr(is_match("ERROR.*: Failed to load blob from .*: Permission denied"))
        .stdout(match_nothing_scanned());
}

#[test]
fn scan_git_emptyrepo() {
    let scan_env = ScanEnv::new();

    let repo = scan_env.input_dir("input_repo");
    create_empty_git_repo(repo.path());

    let path = format!("file://{}", repo.display());
    noseyparker_success!("scan", "-d", scan_env.dspath(), path)
        .stdout(is_match(r"(?m)^Scanned .* from \d+ blobs in .*; 0/0 new matches$"));
}

#[test]
fn scan_secrets1() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file_with_secret("input.txt");

    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("104 B", 1, 1, 1));

    assert_cmd_snapshot!(noseyparker_success!("summarize", "-d", scan_env.dspath()));

    with_settings!({
        filters => get_report_stdout_filters(),
    }, {
        assert_cmd_snapshot!(noseyparker_success!("report", "-d", scan_env.dspath()));
    });

    let cmd = noseyparker_success!("report", "-d", scan_env.dspath(), "--format=json");
    let json_output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
    with_settings!({
        redactions => get_report_json_redactions()
    }, {
        assert_json_snapshot!(json_output);
    });
}

#[test]
fn scan_default_datastore() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file("input.txt");

    let ds = scan_env.root.child("datastore.np");
    ds.assert(predicates::path::missing());

    // first scan with the default datastore
    noseyparker!("scan", input.path())
        .current_dir(scan_env.root.path())
        .assert()
        .success()
        .stdout(match_scan_stats("0 B", 1, 0, 0));

    ds.assert(predicates::path::is_dir());
    input.assert(predicates::path::is_file());

    // Make sure that summarization and reporting works without an explicit datastore
    let cmd = noseyparker!("report", "--format=json")
        .current_dir(scan_env.root.path())
        .assert()
        .success();
    let json_output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
    assert_json_snapshot!(json_output);

    let cmd = noseyparker!("summarize", "--format=json")
        .current_dir(scan_env.root.path())
        .assert()
        .success();
    let json_output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
    assert_json_snapshot!(json_output);

    // now try to scan again with the existing default datastore
    assert_cmd_snapshot!(noseyparker!("scan", input.path())
        .current_dir(scan_env.root.path())
        .assert()
        .failure());

    // Finally, try to scan again with the existing default datastore, explicitly specifying it
    noseyparker!("scan", "-d", ds.path(), input.path())
        .current_dir(scan_env.root.path())
        .assert()
        .success()
        .stdout(match_scan_stats("0 B", 1, 0, 0));
}

#[test]
fn summarize_nonexistent_default_datastore() {
    let scan_env = ScanEnv::new();
    let ds = scan_env.root.child("datastore.np");
    ds.assert(predicates::path::missing());

    assert_cmd_snapshot!(noseyparker!("summarize")
        .current_dir(scan_env.root.path())
        .assert()
        .failure());

    ds.assert(predicates::path::missing());
}

#[test]
fn report_nonexistent_default_datastore() {
    let scan_env = ScanEnv::new();
    let ds = scan_env.root.child("datastore.np");
    ds.assert(predicates::path::missing());

    assert_cmd_snapshot!(noseyparker!("report")
        .current_dir(scan_env.root.path())
        .assert()
        .failure());

    ds.assert(predicates::path::missing());
}

/// Test that the `report` command's `--max-matches` can be given a negative value (which means "no
/// limit" for the option) without requiring an equals sign for the value. That is, instead of
/// _requiring_ that the option be written `--max-matches=-1`, it should work fine to write
/// `--max-matches -1`.
///
/// N.B., Suppoorting that argument parsing requires passing the `allow_negative_numbers=true` in
/// the correct spot in the `clap` code.
#[test]
fn report_unlimited_matches() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file_with_secret("input.txt");

    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("104 B", 1, 1, 1));

    with_settings!({
        filters => get_report_stdout_filters(),
    }, {
        assert_cmd_snapshot!(noseyparker_success!("report", "-d", scan_env.dspath(), "--max-matches", "-1"));
    });
}

/// Test that the `report` command uses colors as expected when *not* running under a pty:
///
/// - When running with the output explicitly written to a file, colors are not used
///
/// - When running with with the output explicitly written to a file and `--color=always`
///   specified, colors are used
#[test]
fn report_output_colors1() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file_with_secret("input.txt");

    let output1 = scan_env.child("findings.txt");
    let output2 = scan_env.child("findings.colored.txt");

    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("104 B", 1, 1, 1));

    noseyparker_success!("report", "-d", scan_env.dspath(), "-o", output1.path());
    noseyparker_success!("report", "-d", scan_env.dspath(), "-o", output2.path(), "--color=always");

    let output1_contents = std::fs::read_to_string(output1.path()).unwrap();
    let output2_contents = std::fs::read_to_string(output2.path()).unwrap();

    assert_ne!(output1_contents, output2_contents);
    with_settings!({
        filters => get_report_stdout_filters(),
    }, {
        assert_snapshot!(output1_contents);
    });
    assert_eq!(&output1_contents, &console::strip_ansi_codes(&output2_contents));
}

// Test that the `report` command uses colors as expected when running under a pty:
// - When running with the output going to stdout (default), colors are used
// - When running with the explicitly written to a file, colors are not used
// XXX to get a pty, look at the `pty-process` crate: https://docs.rs/pty-process/latest/pty_process/blocking/struct.Command.html
