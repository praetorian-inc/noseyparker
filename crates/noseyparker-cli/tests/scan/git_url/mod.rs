use super::*;

#[test]
fn https_nonexistent() {
    let scan_env = ScanEnv::new();

    let path = "https://example.com/nothere.git";
    noseyparker_failure!("scan", "-d", scan_env.dspath(), "--git-url", path)
        .stderr(is_match(r"(?m)^Cloning into bare repository .*$"))
        .stderr(is_match(r"(?m)^fatal: (repository .* not found$|unable to access .*$)"))
        .stderr(is_match(r"(?m)^Error: No inputs to scan$"));
}

// Test what happens when there is no `git` binary but it is needed
#[test]
fn git_binary_missing() {
    let scan_env = ScanEnv::new();

    let path = "https://github.com/praetorian-inc/noseyparker";
    noseyparker!("scan", "-d", scan_env.dspath(), "--git-url", path)
        .env("PATH", "/dev/null")
        .assert()
        .failure()
        .stderr(is_match(r"Failed to clone .*: git execution failed:"))
        .stderr(is_match(r"(?m)^Error: No inputs to scan$"));
}

#[test]
fn ssh_scheme() {
    let scan_env = ScanEnv::new();
    let path = "ssh://example.com/nothere.git";
    assert_cmd_snapshot!(noseyparker_failure!("scan", "-d", scan_env.dspath(), "--git-url", path));
}

#[test]
fn http_scheme() {
    let scan_env = ScanEnv::new();
    let path = "http://example.com/nothere.git";
    assert_cmd_snapshot!(noseyparker_failure!("scan", "-d", scan_env.dspath(), "--git-url", path));
}

#[test]
fn file_scheme() {
    let scan_env = ScanEnv::new();
    let path = "file://example.com/nothere.git";
    assert_cmd_snapshot!(noseyparker_failure!("scan", "-d", scan_env.dspath(), "--git-url", path));
}

#[test]
fn no_scheme() {
    let scan_env = ScanEnv::new();
    let path = "nothere.git";
    assert_cmd_snapshot!(noseyparker_failure!("scan", "-d", scan_env.dspath(), "--git-url", path));
}
