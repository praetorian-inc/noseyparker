use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::Ruleset;

/// A collection of rulesets
#[derive(Serialize, Deserialize)]
pub struct Rulesets {
    pub rulesets: Vec<Ruleset>,
}

impl Rulesets {
    pub fn from_paths_and_contents<'a, I: IntoIterator<Item = (&'a Path, &'a [u8])>>(
        iterable: I,
    ) -> Result<Self> {
        let mut rulesets = Rulesets {
            rulesets: Vec::new(),
        };
        for (path, contents) in iterable.into_iter() {
            let rs: Self = serde_yaml::from_reader(contents)
                .with_context(|| format!("Failed to load rulesets YAML from {}", path.display()))?;
            rulesets.extend(rs);
        }

        Ok(rulesets)
    }

    /// Create an empty collection of rulesets.
    pub fn new() -> Self {
        Self {
            rulesets: Vec::new(),
        }
    }

    /// How many rulesets are in this collection?
    #[inline]
    pub fn len(&self) -> usize {
        self.rulesets.len()
    }

    /// Is this collection of rulesets empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rulesets.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Ruleset> {
        self.rulesets.iter()
    }
}

/// Creates an empty collection of rulesets.
impl Default for Rulesets {
    fn default() -> Self {
        Self::new()
    }
}

impl Extend<Ruleset> for Rulesets {
    fn extend<T: IntoIterator<Item = Ruleset>>(&mut self, iter: T) {
        self.rulesets.extend(iter);
    }
}

impl IntoIterator for Rulesets {
    type Item = Ruleset;
    type IntoIter = <Vec<Ruleset> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.rulesets.into_iter()
    }
}
