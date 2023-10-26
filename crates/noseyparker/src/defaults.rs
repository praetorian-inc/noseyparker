use anyhow::Result;
use include_dir::{include_dir, Dir};
use std::path::Path;

use noseyparker_rules::Rules;

// NOTE: `include_dir` does not seem to play nicely with incremental builds. When adding new rules,
// it seems like this macro does not get re-run, and the files are not added.
//
// This appears to be an issue when not using rust stable; when using nightly, `include_dir!` uses
// the `tracked_path` feature to record the dependencies:
//
// https://doc.rust-lang.org/nightly/proc_macro/tracked_path/index.html
//
pub static DEFAULT_RULES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/data/default/rules");

pub static DEFAULT_IGNORE_RULES: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/default/ignore.conf"));

/// Load the default YAML rule files, returning their pathnames and contents.
pub fn get_default_rule_files() -> Vec<(&'static Path, &'static [u8])> {
    let mut yaml_files: Vec<(&'_ Path, &'_ [u8])> = DEFAULT_RULES_DIR
        .find("**/*.yml")
        .expect("Constant glob should compile")
        .filter_map(|e| e.as_file())
        .map(|f| (f.path(), f.contents()))
        .collect();
    yaml_files.sort_by_key(|t| t.0);
    yaml_files
}

/// Load the default set of rules.
pub fn get_default_rules() -> Result<Rules> {
    Rules::from_paths_and_contents(get_default_rule_files())
}
