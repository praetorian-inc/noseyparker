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

pub static DEFAULT_RULES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/data/default/builtin");

pub static DEFAULT_IGNORE_RULES: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/default/ignore.conf"));

fn load_yaml_files<'a>(dir: &Dir<'a>) -> Vec<(&'a Path, &'a [u8])> {
    dir.find("**/*.yml")
        .expect("Constant glob should compile")
        .filter_map(|e| e.as_file())
        .map(|f| (f.path(), f.contents()))
        .collect()
}

/// Load the default YAML rule files, returning their pathnames and contents.
fn get_default_rule_files() -> Vec<(&'static Path, &'static [u8])> {
    let mut yaml_files = load_yaml_files(&DEFAULT_RULES_DIR);
    yaml_files.sort_by_key(|t| t.0);
    yaml_files
}

/// Load the default rules and rulesets.
pub fn get_builtin_rules() -> Result<Rules> {
    Rules::from_paths_and_contents(get_default_rule_files())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_default_rules() {
        assert!(get_builtin_rules().unwrap().num_rules() >= 100);
    }
}
