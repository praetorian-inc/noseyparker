use super::*;

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
        .stdout(match_scan_stats("0B", 1, 0, 0));
}

#[test]
fn scan_emptyfiles() {
    let scan_env = ScanEnv::new();
    let input1 = scan_env.input_file("empty_file1");
    let input2 = scan_env.input_file("empty_file2");
    noseyparker_success!("scan", "--datastore", scan_env.dspath(), input1.path(), input2.path())
        .stdout(match_scan_stats("0B", 2, 0, 0));
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

// NOTE: this one fails if you are running as root
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
        .stdout(is_match("ERROR.*: Failed to load blob from .*: Permission denied"))
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
        .stdout(match_scan_stats("81B", 1, 1, 1));

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
