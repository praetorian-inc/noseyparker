use anyhow::{bail, Context, Result};
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, debug_span};

use crate::{util, RuleSyntax, RulesetSyntax};

/// A collection of rules and rulesets
#[derive(Serialize, Deserialize, Clone)]
pub struct Rules {
    #[serde(default)]
    pub rules: Vec<RuleSyntax>,

    #[serde(default)]
    pub rulesets: Vec<RulesetSyntax>,
}

impl Rules {
    /// Create an empty collection of rules and rulesets.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rulesets: Vec::new(),
        }
    }

    /// Update this collection of rules by adding those from another collection.
    pub fn update(&mut self, other: Rules) {
        self.rules.extend(other.rules);
        self.rulesets.extend(other.rulesets);
    }

    // Load from an iterable of `(path, contents)`.
    pub fn from_paths_and_contents<'a, I: IntoIterator<Item = (&'a Path, &'a [u8])>>(
        iterable: I,
    ) -> Result<Self> {
        let mut rules = Self::new();
        for (path, contents) in iterable.into_iter() {
            let rs: Self = serde_yaml::from_reader(contents)
                .with_context(|| format!("Failed to load rules YAML from {}", path.display()))?;
            rules.update(rs);
        }

        Ok(rules)
    }

    /// Load rules from the given paths, which may refer either to YAML files or to directories.
    pub fn from_paths<P: AsRef<Path>, I: IntoIterator<Item = P>>(paths: I) -> Result<Self> {
        let mut num_paths = 0;
        let mut rules = Rules::new();
        for input in paths {
            num_paths += 1;
            let input = input.as_ref();
            if input.is_file() {
                rules.update(Rules::from_yaml_file(input)?);
            } else if input.is_dir() {
                rules.update(Rules::from_directory(input)?);
            } else {
                bail!("Unhandled input type: {} is neither a file nor directory", input.display());
            }
        }
        debug!(
            "Loaded {} rules and {} rulesets from {num_paths} paths",
            rules.num_rules(),
            rules.num_rulesets()
        );
        Ok(rules)
    }

    /// Load rules from the given YAML file.
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let _span = debug_span!("Rules::from_yaml_file", "{}", path.display()).entered();
        let rules: Self = util::load_yaml_file(path)
            .with_context(|| format!("Failed to load rules YAML from {}", path.display()))?;
        debug!(
            "Loaded {} rules and {} rulesets from {}",
            rules.num_rules(),
            rules.num_rulesets(),
            path.display()
        );
        Ok(rules)
    }

    /// Load rules from the given YAML files.
    pub fn from_yaml_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(paths: I) -> Result<Self> {
        let mut num_paths = 0;
        let mut rules = Rules::new();
        for path in paths {
            num_paths += 1;
            rules.update(Rules::from_yaml_file(path.as_ref())?);
        }
        debug!(
            "Loaded {} rules and {} rulesets from {num_paths} paths",
            rules.num_rules(),
            rules.num_rulesets()
        );
        Ok(rules)
    }

    /// Load rules from YAML files found recursively within the given directory.
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let _span = debug_span!("Rules::from_directory", "{}", path.display()).entered();

        let yaml_types = TypesBuilder::new().add_defaults().select("yaml").build()?;

        let walker = WalkBuilder::new(path)
            .types(yaml_types)
            .follow_links(true)
            .standard_filters(false)
            .build();
        let mut yaml_files = Vec::new();
        for entry in walker {
            let entry = entry?;
            if entry.file_type().map_or(false, |t| !t.is_dir()) {
                yaml_files.push(entry.into_path());
            }
        }
        yaml_files.sort();
        debug!("Found {} rules files to load within {}", yaml_files.len(), path.display());

        Self::from_yaml_files(&yaml_files)
    }

    /// How many rules are in this collection?
    #[inline]
    pub fn num_rules(&self) -> usize {
        self.rules.len()
    }

    /// How many rulesets are in this collection?
    #[inline]
    pub fn num_rulesets(&self) -> usize {
        self.rulesets.len()
    }

    /// Is this collection of rules and rulesets empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.rulesets.is_empty()
    }

    #[inline]
    pub fn iter_rules(&self) -> std::slice::Iter<'_, RuleSyntax> {
        self.rules.iter()
    }

    #[inline]
    pub fn iter_rulesets(&self) -> std::slice::Iter<'_, RulesetSyntax> {
        self.rulesets.iter()
    }
}

/// Creates an empty collection of rules.
impl Default for Rules {
    fn default() -> Self {
        Self::new()
    }
}
