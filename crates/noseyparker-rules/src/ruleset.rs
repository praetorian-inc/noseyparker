use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, debug_span};

use crate::util;

/// A syntactic representation describing a set of Nosey Parker rules.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct RulesetSyntax {
    /// A unique identifier for this ruleset
    pub id: String,

    /// A human-readable name for the ruleset
    pub name: String,

    /// A description of the ruleset
    pub description: String,

    /// A list of rule IDs included in the ruleset
    pub include_rule_ids: Vec<String>,
}

impl RulesetSyntax {
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
        self.include_rule_ids.len()
    }
}
