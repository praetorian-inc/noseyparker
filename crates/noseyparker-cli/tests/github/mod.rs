//! Tests for Nosey Parker's `github` command

use super::*;
use pretty_assertions::assert_eq;

#[test]
fn github_repos_list_noargs() {
    assert_cmd_snapshot!(noseyparker_failure!("github", "repos", "list"));
}

#[test]
fn github_repos_list_org_badtoken() {
    let cmd = noseyparker!()
        .args(&["github", "repos", "list", "--org", "praetorian-inc"])
        .env("NP_GITHUB_TOKEN", "hahabogus")
        .assert()
        .failure();
    assert_cmd_snapshot!(cmd);
}

#[test]
fn github_repos_list_user_badtoken() {
    let cmd = noseyparker!()
        .args(&["github", "repos", "list", "--user", "octocat"])
        .env("NP_GITHUB_TOKEN", "hahabogus")
        .assert()
        .failure();
    assert_cmd_snapshot!(cmd);
}

// XXX Note: `octocat` is not a user under our control; it's a kind of test account owned by GitHub.
// We are making some assumptions about the `octocat` user's list of repositories that may change.

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

// XXX this assumes that Spoon-Knife will be in the octocat user's repo list
#[test]
fn github_repos_list_user_human_format() {
    let mut cmd = noseyparker!("github", "repos", "list", "--user", "octocat");
    handle_github_token(&mut cmd);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("https://github.com/octocat/Spoon-Knife.git"))
        .stderr(predicate::str::is_empty());
}

// XXX this assumes that Spoon-Knife will be in the octocat user's repo list
#[test]
fn github_repos_list_user_jsonl_format() {
    let mut cmd = noseyparker!("github", "repos", "list", "--user", "octocat", "--format", "jsonl");
    handle_github_token(&mut cmd);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"https://github.com/octocat/Spoon-Knife.git\"\n"))
        .stderr(predicate::str::is_empty());
}

// XXX this assumes that Spoon-Knife will be in the octocat user's non-fork repo list, and linguist will be in its fork repo list
#[test]
fn github_repos_list_user_repo_filter() {
    let mut cmd = noseyparker!(
        "github",
        "repos",
        "list",
        "--user=octocat",
        "--format=jsonl",
        "--repo-type=fork"
    );
    handle_github_token(&mut cmd);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"https://github.com/octocat/linguist.git\"\n"))
        .stderr(predicate::str::is_empty());

    let mut cmd = noseyparker!(
        "github",
        "repos",
        "list",
        "--user=octocat",
        "--format=jsonl",
        "--repo-type=source"
    );
    handle_github_token(&mut cmd);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"https://github.com/octocat/linguist.git\"\n").not())
        .stderr(predicate::str::is_empty());
}

#[test]
fn github_repos_list_multiple_user_dedupe_jsonl_format() {
    let mut cmd = noseyparker!(
        "github", "repos", "list", "--user", "octocat", "--user", "octocat", "--format", "jsonl"
    );
    handle_github_token(&mut cmd);
    let cmd = cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"https://github.com/octocat/Spoon-Knife.git\"\n"))
        .stderr(predicate::str::is_empty());

    // Ensure that output is sorted and there are no dupes
    let stdout = String::from_utf8(cmd.get_output().stdout.clone())
        .expect("noseyparker output should be utf-8");
    let stdout_lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
    let mut sorted_stdout_lines = stdout_lines.clone();
    sorted_stdout_lines.sort();
    sorted_stdout_lines.dedup();
    assert_eq!(stdout_lines, sorted_stdout_lines);
}

#[test]
fn github_repos_list_user_json_format() {
    let mut cmd = noseyparker!("github", "repos", "list", "--user", "octocat", "--format", "json");
    handle_github_token(&mut cmd);
    let cmd = cmd.assert().success().stderr(predicate::str::is_empty());

    let output = &cmd.get_output().stdout;
    let json_parsed: Vec<String> =
        serde_json::from_slice(output).expect("output should be well-formed JSON");
    assert!(
        json_parsed.contains(&String::from("https://github.com/octocat/Spoon-Knife.git")),
        "JSON output does not contain https://github.com/octocat/Spoon-Knife.git: {json_parsed:?}"
    );
}

#[test]
fn github_repos_list_all_organizations_no_api_url1() {
    assert_cmd_snapshot!(noseyparker_failure!(
        "github",
        "repos",
        "list",
        "--all-github-organizations"
    ));
}

#[test]
fn github_repos_list_all_organizations_no_api_url2() {
    assert_cmd_snapshot!(noseyparker_failure!("github", "repos", "list", "--all-organizations"));
}

#[test]
fn github_repos_list_all_organizations_no_api_url3() {
    assert_cmd_snapshot!(noseyparker_failure!(
        "github",
        "repos",
        "list",
        "--all-organizations",
        "--api-url",
        "https://api.github.com/"
    ));
}

#[test]
fn github_repos_list_all_organizations_no_api_url4() {
    assert_cmd_snapshot!(noseyparker_failure!(
        "github",
        "repos",
        "list",
        "--all-organizations",
        "--api-url",
        "https://api.github.com"
    ));
}

// TODO(test): add tests for `github repos list --all-organizations` with a valid non-default `--github-api-url`
// TODO(test): add test using a non-default `--github-api-url URL`
