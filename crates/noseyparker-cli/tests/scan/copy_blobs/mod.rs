use super::*;

#[test]
fn scan_copy_blobs_files_matching() {
    let scan_env = scan_copy_blobs_common("--copy-blobs=matching", "--copy-blobs-format=files");
    let (paths, blobs_dir) = list_blob_paths(&scan_env);
    assert_eq!(
        paths,
        [
            blobs_dir.join("65"),
            blobs_dir.join("65/e7948dcb965586ada5d231709c767c6b8ff3df"),
            blobs_dir.join("be"),
            blobs_dir.join("be/f17e1f92978931020b423cfcfb6f1e7381d559")
        ]
    );
}

#[cfg(feature = "parquet")]
#[test]
fn scan_copy_blobs_parquet_matching() {
    let scan_env = scan_copy_blobs_common("--copy-blobs=matching", "--copy-blobs-format=parquet");
    let (paths, blobs_dir) = list_blob_paths(&scan_env);
    assert_eq!(
        paths,
        [
            blobs_dir.join("blobs.00.parquet"),
            blobs_dir.join("blobs.01.parquet")
        ]
    );
    assert_eq!(
        get_parquet_blob_ids(&paths),
        vec![
            "65e7948dcb965586ada5d231709c767c6b8ff3df",
            "bef17e1f92978931020b423cfcfb6f1e7381d559",
        ]
    );
}

#[cfg(feature = "parquet")]
#[test]
fn scan_copy_blobs_parquet_all_reused_datastore() {
    let copy_blobs = "--copy-blobs=all";
    let copy_blobs_format = "--copy-blobs-format=parquet";

    let scan_env = scan_copy_blobs_common(copy_blobs, copy_blobs_format);
    let (paths, blobs_dir) = list_blob_paths(&scan_env);
    assert_eq!(
        paths,
        [
            blobs_dir.join("blobs.00.parquet"),
            blobs_dir.join("blobs.01.parquet")
        ]
    );
    assert_eq!(
        get_parquet_blob_ids(&paths),
        vec![
            "1fae2bccda861986f8290364d00fb709d7381e81",
            "65e7948dcb965586ada5d231709c767c6b8ff3df",
            "bef17e1f92978931020b423cfcfb6f1e7381d559",
        ]
    );

    // use only 2 of the original 3 paths this time
    let i1 = scan_env.child("i1.txt");
    let i3 = scan_env.child("i3.txt");

    noseyparker_success!(
        "scan",
        "-d",
        scan_env.dspath(),
        "--jobs=2",
        copy_blobs,
        copy_blobs_format,
        i1.path(),
        i3.path()
    )
    .stdout(match_scan_stats("119 B", 2, 0, 3));

    let (paths, blobs_dir) = list_blob_paths(&scan_env);
    assert_eq!(
        paths,
        [
            blobs_dir.join("blobs.00.parquet"),
            blobs_dir.join("blobs.01.parquet"),
            blobs_dir.join("blobs.02.parquet"),
            blobs_dir.join("blobs.03.parquet"),
        ]
    );
    assert_eq!(
        get_parquet_blob_ids(&paths),
        vec![
            "1fae2bccda861986f8290364d00fb709d7381e81",
            "65e7948dcb965586ada5d231709c767c6b8ff3df",
            "bef17e1f92978931020b423cfcfb6f1e7381d559",
        ]
    );
}

#[test]
fn scan_copy_blobs_files_all() {
    let scan_env = scan_copy_blobs_common("--copy-blobs=all", "--copy-blobs-format=files");
    let (paths, blobs_dir) = list_blob_paths(&scan_env);
    assert_eq!(
        paths,
        [
            blobs_dir.join("1f"),
            blobs_dir.join("1f/ae2bccda861986f8290364d00fb709d7381e81"),
            blobs_dir.join("65"),
            blobs_dir.join("65/e7948dcb965586ada5d231709c767c6b8ff3df"),
            blobs_dir.join("be"),
            blobs_dir.join("be/f17e1f92978931020b423cfcfb6f1e7381d559"),
        ]
    );
}

#[test]
fn scan_copy_blobs_files_none() {
    let scan_env = scan_copy_blobs_common("--copy-blobs=none", "--copy-blobs-format=files");
    let (paths, _blobs_dir) = list_blob_paths(&scan_env);
    assert_eq!(paths, Vec::<PathBuf>::new());
}

#[cfg(feature = "parquet")]
#[test]
fn scan_copy_blobs_parquet_none() {
    let scan_env = scan_copy_blobs_common("--copy-blobs=none", "--copy-blobs-format=parquet");
    let (paths, _blobs_dir) = list_blob_paths(&scan_env);
    assert_eq!(paths, Vec::<PathBuf>::new());
}

fn scan_copy_blobs_common(copy_blobs: &str, copy_blobs_format: &str) -> ScanEnv {
    let scan_env = ScanEnv::new();
    let input = scan_env.input_with_secret();
    let i1 = scan_env.input_file_with_contents("i1.txt", input);
    let i2 = scan_env.input_file_with_contents("i2.txt", &format!("{input}\nand again:\n{input}"));
    let i3 = scan_env.input_file_with_contents("i3.txt", "no secrets here");

    noseyparker_success!(
        "scan",
        "-d",
        scan_env.dspath(),
        "--jobs=2",
        copy_blobs,
        copy_blobs_format,
        i1.path(),
        i2.path(),
        i3.path()
    )
    .stdout(match_scan_stats("339 B", 3, 3, 3));

    scan_env
}

/// List the paths of all the blobs in the blobs directory, sorted.
fn list_blob_paths(scan_env: &ScanEnv) -> (Vec<std::path::PathBuf>, std::path::PathBuf) {
    let mut paths = Vec::new();
    let blobs_dir = scan_env.dspath().join("blobs");
    for entry in glob::glob(&format!("{}/**/*", blobs_dir.display())).unwrap() {
        let path = entry.unwrap();
        paths.push(path);
    }
    paths.sort();
    (paths, blobs_dir)
}

#[cfg(feature = "parquet")]
/// Get the list of blob IDs from the given parquet files.
fn get_parquet_blob_ids(paths: &[std::path::PathBuf]) -> Vec<String> {
    let mut ids = Vec::new();
    for path in paths {
        let file = std::fs::File::open(path).unwrap();
        let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();
        for batch in reader {
            let batch = batch.unwrap();
            if let Some(array) = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
            {
                ids.extend(array.iter().map(|s| s.unwrap_or_default().to_string()));
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}
