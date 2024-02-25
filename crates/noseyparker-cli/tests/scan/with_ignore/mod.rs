use indoc::indoc;

use super::*;

// FIXME: this test passes, but does demonstrates that the undesirable thing is done!
// Ignore file entries should be applied to the input roots also.
#[should_panic]
#[test]
fn root_input_noignore_01() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        input.dat
    "#},
    );

    let input = scan_env.input_file_with_secret("input.dat");

    noseyparker_success!(
        "scan",
        "-d",
        scan_env.dspath(),
        "--ignore",
        ignore_file.path(),
        input.path()
    )
    .stdout(match_nothing_scanned());
}

// FIXME: this test passes, but does demonstrates that the undesirable thing is done!
// Ignore file entries should be applied to the input roots also.
#[should_panic]
#[test]
fn root_input_noignore_02() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        input
    "#},
    );

    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/input.dat");

    noseyparker_success!(
        "scan",
        "--ignore",
        ignore_file.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_nothing_scanned());
}

#[test]
fn literal_match_01() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        input.dat
    "#},
    );

    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/input.dat");

    noseyparker_success!("scan", "-i", ignore_file.path(), "-d", scan_env.dspath(), input.path())
        .stdout(match_nothing_scanned());
}

#[test]
fn literal_match_02() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        whoohaw/input.dat
    "#},
    );

    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/input.dat");

    noseyparker_success!(
        "scan",
        input.path(),
        "--ignore",
        ignore_file.path(),
        "-d",
        scan_env.dspath()
    )
    .stdout(match_scan_stats("104 B", 1, 1, 1));
}

#[test]
fn literal_match_03() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        subdir1/
    "#},
    );

    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/subdir1/input.dat");

    noseyparker_success!(
        "scan",
        input.path(),
        "--ignore",
        ignore_file.path(),
        "-d",
        scan_env.dspath()
    )
    .stdout(match_nothing_scanned());
}

#[test]
fn glob_01() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        # here is a comment
        *.dat
    "#},
    );

    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/input.dat");
    scan_env.input_file_with_secret("input/input.txt");
    scan_env.input_file_with_secret("input/subdir/input.dat");

    noseyparker_success!(
        "scan",
        "--ignore",
        ignore_file.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_scan_stats("104 B", 1, 1, 1));
}

#[test]
fn path_glob_01() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        **/test
    "#},
    );

    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/test/input.dat");
    scan_env.input_file_with_secret("input/subdir1/test/input.dat");
    scan_env.input_file_with_secret("input/subdir1/test/subdir2/input.dat");

    noseyparker_success!(
        "scan",
        "--ignore",
        ignore_file.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_nothing_scanned());
}

#[test]
fn negation_01() {
    let scan_env = ScanEnv::new();
    let ignore_file = scan_env.input_file_with_contents(
        "npignore",
        indoc! {r#"
        *.dat
        !**/subdir1/**
    "#},
    );

    let input = scan_env.input_dir("input");
    scan_env.input_file_with_secret("input/input.dat");
    scan_env.input_file_with_secret("input/subdir1/input.dat");
    scan_env.input_file_with_secret("input/subdir2/input.dat");

    noseyparker_success!(
        "scan",
        "-vvv",
        "--ignore",
        ignore_file.path(),
        "-d",
        scan_env.dspath(),
        input.path()
    )
    .stdout(match_scan_stats("104 B", 1, 1, 1));
}
