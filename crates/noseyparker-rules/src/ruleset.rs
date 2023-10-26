use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, debug_span};

use crate::util;

/// A syntactic representation describing a set of Nosey Parker rules.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ruleset {
    /// A description of this ruleset
    pub description: String,

    /// A list of rule IDs to include in this set
    pub include_ids: Vec<String>,
}

impl Ruleset {
    /// Load a ruleset from the given YAML file.
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let _span = debug_span!("Ruleset::from_yaml_file", "{}", path.display()).entered();
        let ruleset: Self = util::load_yaml_file(path)
            .with_context(|| format!("Failed to load ruleset YAML from {}", path.display()))?;
        debug!("Loaded ruleset of {} rules from {}", ruleset.num_rules(), path.display());
        Ok(ruleset)
    }

    /// How many rules are listed in this ruleset?
    pub fn num_rules(&self) -> usize {
        self.include_ids.len()
    }
}
