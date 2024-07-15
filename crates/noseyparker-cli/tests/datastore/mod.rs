use super::*;

#[test]
fn init() {
    let scan_env = ScanEnv::new();
    assert_cmd_snapshot!(noseyparker_success!("datastore", "init", "-d", scan_env.dspath()));
}

/// Create a datastore, export it, extract it, and test that Nosey Parker still sees it as a valid
/// datastore.
#[test]
fn export_empty() {
    let scan_env = ScanEnv::new();
    // create datastore
    noseyparker_success!("datastore", "init", "-d", scan_env.dspath());

    // export it
    let tgz = scan_env.root.child("export.tgz");
    noseyparker_success!("datastore", "export", "-d", scan_env.dspath(), "-o", tgz.path());
    tgz.assert(predicate::path::is_file());

    // extract the archive
    let extract_dir = scan_env.root.child("export.np");
    std::fs::create_dir(&extract_dir).unwrap();

    let file = std::fs::File::open(tgz.path()).unwrap();
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    archive.unpack(&extract_dir).unwrap();

    // make sure the extracted datastore still works
    assert_cmd_snapshot!(noseyparker_success!("summarize", "-d", extract_dir.path()));
}

// TODO: add case for exporting to an already-existing output file
