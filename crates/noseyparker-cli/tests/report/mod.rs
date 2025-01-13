use super::*;

use indoc::indoc;
pub use pretty_assertions::{assert_eq, assert_ne};

#[test]
fn report_nonexistent_default_datastore() {
    let scan_env = ScanEnv::new();
    let ds = scan_env.root.child("datastore.np");
    ds.assert(predicate::path::missing());

    assert_cmd_snapshot!(noseyparker!("report")
        .current_dir(scan_env.root.path())
        .assert()
        .failure());

    ds.assert(predicate::path::missing());
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

fn read_json_file(fname: &Path) -> serde_json::Value {
    let file = std::fs::File::open(fname).unwrap();
    let mut reader = std::io::BufReader::new(file);
    let findings = serde_json::from_reader(&mut reader).unwrap();
    findings
}

/// Test that the `report --finding-status` option works as expected.
/// In the case of a newly-created datastore, there will be no statuses assigned at all, so we do
/// only some basic checks.
#[test]
fn report_finding_status() {
    use serde_json::json;

    let scan_env = ScanEnv::new();
    let input = scan_env.input_file_with_secret("input.txt");
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("104 B", 1, 1, 1));

    let report = |out: &ChildPath, status: &str| {
        noseyparker_success!(
            "report",
            "-d",
            scan_env.dspath(),
            "--format=json",
            "-o",
            out.path(),
            "--finding-status",
            status
        );
    };

    // case 1: accept
    let output = scan_env.child("findings.accept.json");
    report(&output, "accept");
    let findings = read_json_file(output.path());
    assert_eq!(findings, json!([]));

    // case 2: reject
    let output = scan_env.child("findings.reject.json");
    report(&output, "reject");
    let findings = read_json_file(output.path());
    assert_eq!(findings, json!([]));

    // case 3: mixed
    let output = scan_env.child("findings.mixed.json");
    report(&output, "mixed");
    let findings = read_json_file(output.path());
    assert_eq!(findings, json!([]));

    // case 4: null
    let output = scan_env.child("findings.null.json");
    report(&output, "null");
    let findings = read_json_file(output.path());
    assert!(findings.is_array());
    assert_eq!(findings.as_array().unwrap().len(), 1);
}

#[test]
fn max_provenance_exceeded() {
    let scan_env = ScanEnv::new();

    // 4 inputs with the same content
    let i1 = scan_env.input_file_with_secret("i1.txt");
    let i2 = scan_env.input_file_with_secret("i2.txt");
    let i3 = scan_env.input_file_with_secret("i3.txt");
    let i4 = scan_env.input_file_with_secret("i4.txt");

    noseyparker_success!(
        "scan",
        "-d",
        scan_env.dspath(),
        i1.path(),
        i2.path(),
        i3.path(),
        i4.path()
    )
    .stdout(match_scan_stats("416 B", 4, 1, 1));

    // default setting truncates provenance to 3
    {
        let cmd = noseyparker_success!("report", "-d", scan_env.dspath(), "--format=json");
        let output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
        let ps = &output[0]["matches"][0]["provenance"];
        assert!(ps.is_array(), "not an array: {ps:?}");
        assert_eq!(ps.as_array().unwrap().len(), 3);
    }

    // unlimited setting gives us 4
    for limit in ["-1", "0"] {
        let cmd = noseyparker_success!(
            "report",
            "-d",
            scan_env.dspath(),
            "--format=json",
            "--max-provenance",
            limit
        );
        let output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();
        let ps = &output[0]["matches"][0]["provenance"];
        assert!(ps.is_array(), "not an array: {ps:?}");
        assert_eq!(ps.as_array().unwrap().len(), 4);
    }
}

#[test]
fn redundant_matches() {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_file_with_contents(
        "input.txt",
        indoc! {r#"
            aws_access_key_id = 'AKIADEADBEEFDEADBEEF'
            aws_secret_access_key = 'FakeValues99cl9bqJFVA3iFUm+yqVe08HxhXFE/'
        "#},
    );

    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("110 B", 1, 3, 3));

    // Should have only a single finding / match reported by default settings
    {
        let cmd = noseyparker_success!("report", "-d", scan_env.dspath(), "--format=json");
        let output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();

        assert_eq!(output.as_array().unwrap().len(), 1);
        let ms = &output[0]["matches"];
        assert_eq!(ms.as_array().unwrap().len(), 1);
    }

    // Should have 3 findings with 1 match each with `--suppress-redundant=false`
    {
        let cmd = noseyparker_success!(
            "report",
            "-d",
            scan_env.dspath(),
            "--format=json",
            "--suppress-redundant=false"
        );
        let output: serde_json::Value = serde_json::from_slice(&cmd.get_output().stdout).unwrap();

        assert_eq!(output.as_array().unwrap().len(), 3);
        for f in output.as_array().unwrap() {
            assert_eq!(f["matches"].as_array().unwrap().len(), 1);
        }
    }

    // Make sure re-scanning doesn't crash
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("110 B", 1, 0, 3));
}

// Test that the `report` command uses colors as expected when running under a pty:
// - When running with the output going to stdout (default), colors are used
// - When running with the explicitly written to a file, colors are not used
// XXX to get a pty, look at the `pty-process` crate: https://docs.rs/pty-process/latest/pty_process/blocking/struct.Command.html
