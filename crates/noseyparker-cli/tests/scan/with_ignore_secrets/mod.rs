//! Tests for the `--ignore-secrets` option

use indoc::indoc;

use super::*;

/// Test that the default ignore-secrets.conf suppresses AWS example keys
#[test]
fn default_ignore_aws_example_key() {
    let scan_env = ScanEnv::new();

    // Create a file with an AWS example key that should be in the default ignore list
    let input = scan_env.input_file_with_contents(
        "aws_example.txt",
        indoc! {r#"
            # AWS configuration with example key
            aws_access_key_id = AKIAIOSFODNN7EXAMPLE
            aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
        "#},
    );

    // The default ignore-secrets.conf should suppress these known false positives
    // File is still scanned (143 B, 1 blob) but 0 matches because secrets are filtered
    noseyparker_success!("scan", "-d", scan_env.dspath(), input.path())
        .stdout(match_scan_stats("143 B", 1, 0, 0));
}

/// Test that custom ignore-secrets file works
#[test]
fn custom_ignore_secrets_file() {
    let scan_env = ScanEnv::new();

    // Create an ignore-secrets file with the test secret
    let ignore_file = scan_env.input_file_with_contents(
        "ignore-secrets.conf",
        indoc! {r#"
            # Ignore this specific GitHub PAT
            ghp_XIxB7KMNdAr3zqWtQqhE94qglHqOzn1D1stg
        "#},
    );

    // Create input with the secret
    let input = scan_env.input_file_with_secret("input.txt");

    // Should find 0 matches because we ignore the specific secret
    // File is still scanned (104 B, 1 blob) but matches are filtered
    noseyparker_success!(
        "scan",
        "--ignore-secrets",
        ignore_file.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_scan_stats("104 B", 1, 0, 0));
}

/// Test that secrets NOT in the ignore list are still detected
#[test]
fn non_ignored_secret_still_detected() {
    let scan_env = ScanEnv::new();

    // Create an ignore-secrets file with a DIFFERENT secret
    let ignore_file = scan_env.input_file_with_contents(
        "ignore-secrets.conf",
        indoc! {r#"
            # This is a different secret
            ghp_DIFFERENTKEYNOTINFILE0000000000000000
        "#},
    );

    // Create input with a secret that is NOT in the ignore list
    let input = scan_env.input_file_with_secret("input.txt");

    // Should still find the secret
    noseyparker_success!(
        "scan",
        "--ignore-secrets",
        ignore_file.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_scan_stats("104 B", 1, 1, 1));
}

/// Test that multiple --ignore-secrets files can be combined
#[test]
fn multiple_ignore_secrets_files() {
    let scan_env = ScanEnv::new();

    // Create two ignore-secrets files
    let ignore_file1 = scan_env.input_file_with_contents(
        "ignore1.conf",
        indoc! {r#"
            # First ignored secret
            SECRET_ONE_12345
        "#},
    );

    let ignore_file2 = scan_env.input_file_with_contents(
        "ignore2.conf",
        indoc! {r#"
            # Second ignored secret - the actual test secret
            ghp_XIxB7KMNdAr3zqWtQqhE94qglHqOzn1D1stg
        "#},
    );

    // Create input with a secret that is in the second ignore file
    let input = scan_env.input_file_with_secret("input.txt");

    // Should find 0 matches because the secret is in ignore2.conf
    // File is still scanned (104 B, 1 blob) but matches are filtered
    noseyparker_success!(
        "scan",
        "--ignore-secrets",
        ignore_file1.path(),
        "--ignore-secrets",
        ignore_file2.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_scan_stats("104 B", 1, 0, 0));
}

/// Test that comments and empty lines in ignore-secrets files are handled correctly
#[test]
fn ignore_secrets_with_comments() {
    let scan_env = ScanEnv::new();

    let ignore_file = scan_env.input_file_with_contents(
        "ignore-secrets.conf",
        indoc! {r#"
            # This is a comment

            # Another comment with leading whitespace
              # Yet another comment

            ghp_XIxB7KMNdAr3zqWtQqhE94qglHqOzn1D1stg

            # Trailing comment
        "#},
    );

    let input = scan_env.input_file_with_secret("input.txt");

    // File is still scanned (104 B, 1 blob) but matches are filtered
    noseyparker_success!(
        "scan",
        "--ignore-secrets",
        ignore_file.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_scan_stats("104 B", 1, 0, 0));
}

/// Test combining --ignore (path-based) and --ignore-secrets (value-based)
#[test]
fn combine_ignore_and_ignore_secrets() {
    let scan_env = ScanEnv::new();

    // Create path-based ignore file
    let path_ignore = scan_env.input_file_with_contents(
        "path-ignore.conf",
        indoc! {r#"
            # Ignore files named ignored.txt
            ignored.txt
        "#},
    );

    // Create secret-based ignore file
    let secret_ignore = scan_env.input_file_with_contents(
        "secret-ignore.conf",
        indoc! {r#"
            # Ignore this specific secret
            ghp_XIxB7KMNdAr3zqWtQqhE94qglHqOzn1D1stg
        "#},
    );

    // Create input directory with two files:
    // - One that should be ignored by path
    // - One that should be ignored by secret value
    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/ignored.txt"); // Ignored by path
    scan_env.input_file_with_secret("input/scanned.txt"); // Ignored by secret value

    // Should find 0 matches because:
    // - ignored.txt is skipped due to path ignore (not scanned at all)
    // - scanned.txt's secret is in the ignore-secrets list (scanned but filtered)
    // Only scanned.txt is actually scanned (104 B, 1 blob)
    noseyparker_success!(
        "scan",
        "--ignore",
        path_ignore.path(),
        "--ignore-secrets",
        secret_ignore.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_scan_stats("104 B", 1, 0, 0));
}
