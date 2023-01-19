use include_dir::{include_dir, Dir};

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
