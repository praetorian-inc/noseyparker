use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::HashSet;
use tracing::{debug, error, error_span, info, warn};
use vectorscan_rs::{BlockDatabase, Flag, Pattern, Scan};

use noseyparker::rules_database::RulesDatabase;
use noseyparker_rules::{Rule, RulesetSyntax};

use crate::args::{GlobalArgs, RulesCheckArgs};
use crate::rule_loader::RuleLoader;
use crate::util::Counted;

pub fn run(_global_args: &GlobalArgs, args: &RulesCheckArgs) -> Result<()> {
    let _span = error_span!("cmd_rules_check").entered();

    let loaded = RuleLoader::from_rule_specifiers(&args.rules)
        .load()
        .context("Failed to load rules")?;

    let mut rules: Vec<&Rule> = loaded.iter_rules().collect();
    rules.sort_by(|r1, r2| r1.id().cmp(r2.id()));

    let mut rulesets: Vec<&RulesetSyntax> = loaded.iter_rulesets().collect();
    rulesets.sort_by(|r1, r2| r1.id.cmp(&r2.id));

    let mut num_errors = 0;
    let mut num_warnings = 0;

    let id_validator_pat = Regex::new(r"^[a-zA-Z0-9]+(?:[.-][a-zA-Z0-9]+)*$")
        .expect("ID validator pattern should compile");
    const ID_LIMIT: usize = 20;

    // ensure ruleset IDs are globally unique
    {
        let mut seen_ids = HashSet::<&str>::new();
        for ruleset in rulesets.iter() {
            let id = &ruleset.id;
            if !seen_ids.insert(id) {
                error!("Ruleset ID {id} is not unique");
                num_errors += 1;
            }
        }
    }

    // ensure ruleset IDs are well-formed
    {
        for ruleset in rulesets.iter() {
            let id = &ruleset.id;
            let id_len = id.len();
            if id_len > ID_LIMIT {
                error!(
                    "Ruleset ID {id} is too long ({id_len} characters: \
                       should be {ID_LIMIT} characters max)"
                );
                num_errors += 1;
            }

            if !id_validator_pat.is_match(id) {
                error!(
                    "Ruleset ID {id} is not well-formed: \
                       it should consist only of alphanumeric sections \
                       delimited by hyphens or periods"
                );
                num_errors += 1;
            }
        }
    }

    // ensure rule IDs are globally unique
    {
        let mut seen_ids = HashSet::<&str>::new();
        for rule in rules.iter() {
            let id = rule.id();
            if !seen_ids.insert(id) {
                error!("Rule ID {id} is not unique");
                num_errors += 1;
            }
        }
    }

    // ensure rule IDs are well-formed
    {
        for rule in rules.iter() {
            let id = rule.id();
            let id_len = id.len();
            if id_len > ID_LIMIT {
                error!(
                    "Rule ID {id} is too long ({id_len} characters: \
                       should be {ID_LIMIT} characters max)"
                );
                num_errors += 1;
            }

            if !id_validator_pat.is_match(id) {
                error!(
                    "Rule ID {id} is not well-formed: \
                       it should consist only of alphanumeric sections \
                       delimited by hyphens or periods"
                );
                num_errors += 1;
            }
        }
    }

    // ensure that in each ruleset:
    // - all referenced rules resolve
    // - all referenced rules are unique
    {
        for ruleset in rulesets.iter() {
            let _span = error_span!("ruleset", "{}", ruleset.id).entered();
            if let Err(e) = loaded.resolve_ruleset_rules(ruleset) {
                error!("Failed to resolve rules: {e}");
                num_errors += 1;
            }

            let mut seen_ids = HashSet::<&str>::new();
            for id in ruleset.include_rule_ids.iter() {
                if !seen_ids.insert(id) {
                    warn!("Rule ID {id} is not unique");
                    num_warnings += 1;
                }
            }
        }
    }

    // check the rules individually
    for rule in rules.iter() {
        let stats = check_rule(rule, args)?;
        num_errors += stats.num_errors;
        num_warnings += stats.num_warnings;
    }

    // check that every rule is included in at least one ruleset
    {
        let mut seen_rule_ids = HashSet::new();
        for ruleset in rulesets.iter() {
            seen_rule_ids.extend(ruleset.include_rule_ids.iter());
        }

        for rule in rules.iter() {
            let id = &rule.syntax().id;
            if !seen_rule_ids.contains(id) {
                warn!("Rule ID {id} ({}) is not referenced from any known ruleset", rule.name());
                num_warnings += 1;
            }
        }
    }

    // check that the rules can all compile together
    let rules: Vec<Rule> = rules.into_iter().cloned().collect();
    let _rules_db =
        RulesDatabase::from_rules(rules).context("Failed to compile combined rules database")?;

    // XXX: if args.pedantic, should check that all rules compile together with SOM_LEFTMOST

    if num_warnings == 0 && num_errors == 0 {
        println!(
            "{} and {}: no issues detected",
            Counted::regular(loaded.num_rules(), "rule"),
            Counted::regular(loaded.num_rulesets(), "ruleset"),
        );
    } else {
        println!(
            "{} and {}: {num_errors} errors and {num_warnings} warnings",
            Counted::regular(loaded.num_rules(), "rule"),
            Counted::regular(loaded.num_rulesets(), "ruleset"),
        );
    }

    if num_errors != 0 {
        bail!("{}", Counted::regular(num_errors, "error"));
    }

    if num_warnings != 0 && args.warnings_as_errors {
        bail!(
            "{}; warnings being treated as errors",
            Counted::regular(num_warnings, "warning")
        );
    }

    Ok(())
}

fn hs_compile_pattern(pat: &str) -> Result<BlockDatabase> {
    let pat = pat.as_bytes().to_vec();
    let db = BlockDatabase::new(vec![Pattern::new(pat, Flag::default(), None)])?;
    Ok(db)
}

fn hs_compile_pattern_som_leftmost(pat: &str) -> Result<BlockDatabase> {
    let pat = pat.as_bytes().to_vec();
    let db = BlockDatabase::new(vec![Pattern::new(
        pat,
        Flag::default() | Flag::SOM_LEFTMOST,
        None,
    )])?;
    Ok(db)
}

struct CheckStats {
    num_warnings: usize,
    num_errors: usize,
}

fn check_rule(rule: &Rule, args: &RulesCheckArgs) -> Result<CheckStats> {
    let syntax = rule.syntax();
    let _span = error_span!("rule", "{}", syntax.id).entered();

    let mut num_warnings = 0;
    let mut num_errors = 0;

    let num_examples = syntax.examples.len();
    if num_examples == 0 {
        warn!("Rule has no examples");
        num_warnings += 1;
    }

    match syntax.as_regex() {
        Err(e) => {
            error!("Regex: failed to compile pattern: {e}");
            num_errors += 1;
        }
        Ok(pat) => {
            // Check that the rule has at least one capture group
            match pat.static_captures_len() {
                // the default is a single capture group for the entire match
                // not sure if 0 can actually happen
                Some(0) | Some(1) => {
                    error!("Rule has no capture groups");
                    num_errors += 1;
                }
                Some(_len) => {}
                None => {
                    error!("Rule has a variable number of capture groups");
                    num_errors += 1;
                }
            }

            let mut num_succeeded = 0;
            let mut num_failed = 0;

            // Check positive examples
            for (example_num, example) in syntax.examples.iter().enumerate() {
                if pat.find(example.as_bytes()).is_none() {
                    error!("Regex: failed to match example {example_num}: {example:?}");
                    num_failed += 1;
                    num_errors += 1;
                } else {
                    num_succeeded += 1;
                }
            }

            // Check negative examples
            for (example_num, example) in syntax.negative_examples.iter().enumerate() {
                if pat.find(example.as_bytes()).is_some() {
                    error!(
                        "Regex: incorrectly matched negative example {example_num}: {example:?}"
                    );
                    num_failed += 1;
                    num_errors += 1;
                } else {
                    num_succeeded += 1;
                }
            }

            let num_total = num_succeeded + num_failed;
            if num_total > 0 {
                info!("Regex: {num_succeeded}/{num_total} examples succeeded");
            }
        }
    };

    let uncommented_pattern = syntax.uncommented_pattern();
    match hs_compile_pattern(&uncommented_pattern) {
        Err(e) => {
            error!("Vectorscan: failed to compile pattern: {e}");
            num_errors += 1;
        }
        Ok(db) => {
            debug!("{} regex bytes -> {} vectorscan bytes", uncommented_pattern.len(), db.size()?);

            let mut scanner = vectorscan_rs::BlockScanner::new(&db)?;

            let mut num_succeeded = 0;
            let mut num_failed = 0;

            // Check positive examples
            for (example_num, example) in syntax.examples.iter().enumerate() {
                let mut matched = false;
                scanner.scan(example.as_bytes(), |_id, _from, _to, _flags| {
                    matched = true;
                    Scan::Continue
                })?;
                if !matched {
                    error!("Vectorscan: failed to match example {example_num}: {example:?}");
                    num_failed += 1;
                    num_errors += 1;
                } else {
                    num_succeeded += 1;
                }
            }

            // Check negative examples
            for (example_num, example) in syntax.negative_examples.iter().enumerate() {
                let mut matched = false;
                scanner.scan(example.as_bytes(), |_id, _from, _to, _flags| {
                    matched = true;
                    Scan::Continue
                })?;
                if matched {
                    error!("Vectorscan: incorrectly matched negative example {example_num}: {example:?}");
                    num_failed += 1;
                    num_errors += 1;
                } else {
                    num_succeeded += 1;
                }
            }

            let num_total = num_succeeded + num_failed;
            if num_total > 0 {
                info!("Vectorscan: {num_succeeded}/{num_total} examples succeeded");
            }
        }
    }

    if args.pedantic {
        if let Err(e) = hs_compile_pattern_som_leftmost(&syntax.uncommented_pattern()) {
            error!("Vectorscan: failed to compile pattern with start-of-match reporting: {}", e);
            num_errors += 1;
        }
    }

    if args.pedantic && syntax.description.is_none() {
        error!("Rule has no description");
        num_errors += 1;
    }

    if num_warnings == 0 && num_errors == 0 {
        info!("No issues detected");
    } else {
        info!("{num_errors} errors and {num_warnings} warnings");
    }

    Ok(CheckStats {
        num_warnings,
        num_errors,
    })
}
