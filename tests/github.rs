//! Tests for Nosey Parker `github` command

mod common;
use common::*;

#[test]
fn github_repos_list_noargs() {
    assert_cmd_snapshot!(noseyparker_failure!("github", "repos", "list"));
}

#[test]
fn github_repos_list_org_badtoken() {
    let cmd = noseyparker()
        .args(&["github", "repos", "list", "--org", "praetorian-inc"])
        .env("NP_GITHUB_TOKEN", "hahabogus")
        .assert()
        .failure();
    assert_cmd_snapshot!(cmd);
}

#[test]
fn github_repos_list_user_badtoken() {
    let cmd = noseyparker()
        .args(&["github", "repos", "list", "--user", "octocat"])
        .env("NP_GITHUB_TOKEN", "hahabogus")
        .assert()
        .failure();
    assert_cmd_snapshot!(cmd);
}


// XXX Note: `octocat` is not a user under our control; it's a kind of test account owned by GitHub.
// We are assuming that the `octocat` user's list of repositories will always include `Spoon-Knife`.

// XXX Note: the following test cases make actual GitHub requests and may fail due to rate limiting
// issues when not using a token.
//
// To avoid flaky tests, we have these tests use a token from the environment when `CI=1` (set in
// GitHub Actions), and use no token otherwise.
fn handle_github_token(cmd: &mut Command) {
    if std::env::var("CI").is_ok() {
        assert!(std::env::var("NP_GITHUB_TOKEN").is_ok());
    } else {
        cmd.env_remove("NP_GITHUB_TOKEN");
    }
}

#[test]
fn github_repos_list_user_human_format() {
    let mut cmd = noseyparker!("github", "repos", "list", "--user", "octocat");
    handle_github_token(&mut cmd);
    cmd
        .assert()
        .success()
        .stdout(predicates::str::contains("https://github.com/octocat/Spoon-Knife.git"))
        .stderr(predicates::str::is_empty());
}

#[test]
fn github_repos_list_user_jsonl_format() {
    let mut cmd = noseyparker!("github", "repos", "list", "--user", "octocat", "--format", "jsonl");
    handle_github_token(&mut cmd);
    cmd
        .assert()
        .success()
        .stdout(predicates::str::contains("\"https://github.com/octocat/Spoon-Knife.git\"\n"))
        .stderr(predicates::str::is_empty());
}

#[test]
fn github_repos_list_user_json_format() {
    let mut cmd = noseyparker!("github", "repos", "list", "--user", "octocat", "--format", "json");
    handle_github_token(&mut cmd);
    let cmd = cmd
        .assert()
        .success()
        .stderr(predicates::str::is_empty());

    let output = &cmd.get_output().stdout;
    let json_parsed: Vec<String> = serde_json::from_slice(output).expect("output should be well-formed JSON");
    assert!(json_parsed.contains(&String::from("https://github.com/octocat/Spoon-Knife.git")),
        "JSON output does not contain https://github.com/octocat/Spoon-Knife.git: {json_parsed:?}");
}
