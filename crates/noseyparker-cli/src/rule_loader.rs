use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use noseyparker::defaults::get_builtin_rules;
use noseyparker_rules::{Rule, Rules, RulesetSyntax};

use crate::args::RuleSpecifierArgs;
use crate::util::Counted;

pub struct RuleLoader {
    load_builtins: bool,
    additional_load_paths: Vec<PathBuf>,
    enabled_ruleset_ids: Vec<String>,
}

impl RuleLoader {
    /// Create a new loader that loads the builtin rules and rulesets and enables the default
    /// ruleset.
    pub fn new() -> Self {
        Self {
            load_builtins: true,
            additional_load_paths: Vec::new(),
            enabled_ruleset_ids: Vec::new(),
        }
    }

    pub fn load_builtins(mut self, load_builtins: bool) -> Self {
        self.load_builtins = load_builtins;
        self
    }

    /// Add additional file or directory paths to load rules and rulesets from.
    pub fn additional_rule_load_paths<P: AsRef<Path>, I: IntoIterator<Item = P>>(
        mut self,
        paths: I,
    ) -> Self {
        self.additional_load_paths
            .extend(paths.into_iter().map(|p| p.as_ref().to_owned()));
        self
    }

    /// Add additional ruleset IDs to enable.
    pub fn enable_ruleset_ids<S: AsRef<str>, I: IntoIterator<Item = S>>(mut self, ids: I) -> Self {
        self.enabled_ruleset_ids
            .extend(ids.into_iter().map(|p| p.as_ref().to_owned()));
        self
    }

    /// Load rules according to this loader's configuration.
    pub fn load(&self) -> Result<LoadedRules> {
        let mut rules = Rules::new();

        if self.load_builtins {
            let builtin_rules = get_builtin_rules().context("Failed to load builtin rules")?;
            rules.update(builtin_rules);
        }

        if !self.additional_load_paths.is_empty() {
            let custom = Rules::from_paths(&self.additional_load_paths)
                .context("Failed to load rules from additional paths")?;
            rules.update(custom);
        }

        let mut enabled_ruleset_ids = self.enabled_ruleset_ids.clone();

        enabled_ruleset_ids.sort();
        enabled_ruleset_ids.dedup();

        let (mut rules, mut rulesets) = (rules.rules, rules.rulesets);

        rules.sort_by(|r1, r2| r1.id.cmp(&r2.id));
        rulesets.sort_by(|r1, r2| r1.id.cmp(&r2.id));

        let id_to_rule: HashMap<String, Rule> = rules
            .into_iter()
            .map(|r| (r.id.clone(), Rule::new(r)))
            .collect();

        let id_to_ruleset: HashMap<String, RulesetSyntax> =
            rulesets.into_iter().map(|r| (r.id.clone(), r)).collect();

        Ok(LoadedRules {
            id_to_rule,
            id_to_ruleset,
            enabled_ruleset_ids,
        })
    }

    pub fn from_rule_specifiers(specs: &RuleSpecifierArgs) -> Self {
        Self::new()
            .load_builtins(specs.load_builtins)
            .additional_rule_load_paths(specs.rules_path.as_slice())
            .enable_ruleset_ids(specs.ruleset.iter())
    }
}

/// The result of calling `RuleLoader::load`.
pub struct LoadedRules {
    id_to_rule: HashMap<String, Rule>,
    id_to_ruleset: HashMap<String, RulesetSyntax>,

    enabled_ruleset_ids: Vec<String>,
}

impl LoadedRules {
    #[inline]
    pub fn num_rules(&self) -> usize {
        self.id_to_rule.len()
    }

    #[inline]
    pub fn num_rulesets(&self) -> usize {
        self.id_to_ruleset.len()
    }

    /// Get an iterator over the loaded rules.
    /// N.B., the rules are not iterated in any sorted order!
    #[inline]
    pub fn iter_rules(&self) -> impl Iterator<Item = &Rule> {
        self.id_to_rule.values()
    }

    /// Get an iterator over the loaded rulesets.
    /// N.B., the rulesets are not iterated in any sorted order!
    #[inline]
    pub fn iter_rulesets(&self) -> impl Iterator<Item = &RulesetSyntax> {
        self.id_to_ruleset.values()
    }

    /// Get the sorted, deduplicated collection of rules that are enabled according to the
    /// requested rulesets.
    pub fn resolve_enabled_rules(&self) -> Result<Vec<&Rule>> {
        // Check that each mentioned non-special ruleset resolves
        let mut resolved_rulesets: Vec<&RulesetSyntax> = Vec::new();
        let mut all_ruleset_enabled = false;

        for id in self.enabled_ruleset_ids.iter() {
            if id.as_str() == "all" {
                all_ruleset_enabled = true;
            } else if let Some(ruleset) = self.id_to_ruleset.get(id) {
                resolved_rulesets.push(ruleset);
            } else {
                bail!("Unknown ruleset `{id}`");
            }
        }

        // Sort and dedupe the requested rulesets
        resolved_rulesets.sort_by(|r1, r2| r1.id.cmp(&r2.id));
        resolved_rulesets.dedup_by(|r1, r2| r1.id == r2.id);

        // Handle special rulesets specially; resolve other ones normally
        let mut rules: Vec<&Rule> = if all_ruleset_enabled {
            debug!("Using special ruleset `all`");
            self.iter_rules().collect()
        } else {
            let mut rules = Vec::new();
            for &ruleset in resolved_rulesets.iter() {
                rules.extend(self.resolve_ruleset_rules(ruleset)?);
            }

            info!(
                "Loaded {} from {}",
                Counted::regular(rules.len(), "rule"),
                Counted::regular(resolved_rulesets.len(), "ruleset"),
            );

            if tracing::enabled!(tracing::Level::DEBUG) {
                for ruleset in resolved_rulesets {
                    debug!(
                        "Using ruleset `{}`: {} ({})",
                        ruleset.id,
                        ruleset.name,
                        Counted::regular(ruleset.num_rules(), "rule"),
                    );
                }
            }

            rules
        };

        sort_and_deduplicate_rules(&mut rules);

        if tracing::enabled!(tracing::Level::DEBUG) {
            for rule in rules.iter() {
                debug!("Using rule `{}`: {}", rule.id(), rule.name());
            }
        }

        Ok(rules)
    }

    /// Get the sorted, deduplicated collection of rules that are enabled according to the given
    /// ruleset.
    pub fn resolve_ruleset_rules(&self, ruleset: &RulesetSyntax) -> Result<Vec<&Rule>> {
        let mut rules = Vec::new();
        for rule_id in ruleset.include_rule_ids.iter() {
            let rule = self.id_to_rule.get(rule_id).ok_or_else(|| {
                anyhow!("ruleset `{}` ({}): unknown rule `{rule_id}`", ruleset.id, ruleset.name)
            })?;
            rules.push(rule);
        }

        sort_and_deduplicate_rules(&mut rules);

        Ok(rules)
    }
}

/// Deduplicate and sort a collection of rules
fn sort_and_deduplicate_rules(rules: &mut Vec<&Rule>) {
    rules.sort_by(|r1, r2| r1.id().cmp(r2.id()));
    rules.dedup_by(|r1, r2| r1.id() == r2.id());
}
