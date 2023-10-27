use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use noseyparker::defaults::get_default_rules;
use noseyparker_rules::Rules;

use crate::args::RuleSpecifierArgs;


pub struct RuleLoader {
    load_builtin_rules: bool,
    additional_rule_paths: Vec<PathBuf>,
}

impl RuleLoader {
    /// Create a new loader that loads the builtin rules.
    pub fn new() -> Self {
        Self {
            load_builtin_rules: true,
            additional_rule_paths: Vec::new(),
        }
    }

    /*
    /// Configure whether or not to load the builtin rules.
    pub fn load_builtin_rules(mut self, load_builtin_rules: bool) -> Self {
        self.load_builtin_rules = load_builtin_rules;
        self
    }

    /// Add an additional file or directory path to load rules from.
    pub fn additional_rule_path<P: AsRef<Path>>(mut self, p: P) -> Self {
        self.additional_rule_paths.push(p.as_ref().to_owned());
        self
    }
    */

    /// Add additional file or directory paths to load rules from.
    pub fn additional_rule_paths<P: AsRef<Path>, I: IntoIterator<Item = P>>(
        mut self,
        paths: I,
    ) -> Self {
        self.additional_rule_paths
            .extend(paths.into_iter().map(|p| p.as_ref().to_owned()));
        self
    }

    /// Load rules according to this loader's configuration.
    pub fn load(&self) -> Result<Rules> {
        let mut rules = Rules::new();

        if self.load_builtin_rules {
            let builtins = get_default_rules().context("Failed to load default rules")?;
            rules.extend(builtins);
        }

        if !self.additional_rule_paths.is_empty() {
            let custom_rules = Rules::from_paths(&self.additional_rule_paths)
                .context("Failed to load specified rules files")?;
            rules.extend(custom_rules);
        }

        rules.rules.sort_by_key(|r| r.id.clone());

        Ok(rules)
    }

    pub fn from_rule_specifiers(specs: &RuleSpecifierArgs) -> Self {
        Self::new().additional_rule_paths(specs.rules.as_slice())
    }
}
