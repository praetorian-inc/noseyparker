use super::*;

#[test]
fn github_all_orgs_no_api_url() {
    let scan_env = ScanEnv::new();
    assert_cmd_snapshot!(noseyparker_failure!(
        "scan",
        "-d",
        scan_env.dspath(),
        "--all-github-organizations"
    ));
}

#[test]
fn github_all_orgs_explicit_default_api_url() {
    let scan_env = ScanEnv::new();
    assert_cmd_snapshot!(noseyparker_failure!(
        "scan",
        "-d",
        scan_env.dspath(),
        "--all-github-organizations",
        "--github-api-url",
        "https://api.github.com"
    ));
}
