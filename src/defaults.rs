use include_dir::{include_dir, Dir};

pub static DEFAULT_RULES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/data/default/rules");

pub static DEFAULT_IGNORE_RULES: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/default/ignore.conf"));
