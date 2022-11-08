use anyhow::{Context, Result};
use hyperscan::prelude::{pattern, BlockDatabase, Builder, Matching};

use tracing::{debug, debug_span, error, error_span, info, warn};

use crate::args;
use noseyparker::rules::{Rule, Rules};
use noseyparker::rules_database::RulesDatabase;

pub fn run(_global_args: &args::GlobalArgs, args: &args::RulesArgs) -> Result<()> {
    match &args.command {
        args::RulesCommand::Check(args) => cmd_rules_check(args),
    }
}

fn cmd_rules_check(args: &args::RulesCheckArgs) -> Result<()> {
    let _span = debug_span!("cmd_rules_check").entered();

    let rules = Rules::from_paths(&args.inputs)?;
    let mut num_errors = 0;
    let mut num_warnings = 0;
    let num_rules = rules.rules.len();
    for (rule_num, rule) in rules.rules.iter().enumerate() {
        let stats = check_rule(rule_num, rule)?;
        num_errors += stats.num_errors;
        num_warnings += stats.num_warnings;
    }
    let _rules_db = RulesDatabase::from_rules(rules)
        .context("Compiling rules database failed")?;

    if num_warnings == 0 && num_errors == 0 {
        info!("{} rules: no issues detected", num_rules);
    } else {
        info!("{} rules: {} errors and {} warnings", num_rules, num_errors, num_warnings);
    }

    Ok(())
}

fn hs_compile_pattern(pat: &str) -> Result<BlockDatabase> {
    let pattern = pattern! {pat};
    let db: BlockDatabase = pattern.build()?;
    Ok(db)
}

// fn hs_compile_pattern_streaming(pat: &str) -> Result<StreamingDatabase> {
//     let pattern = pattern!{pat};
//     let mut pattern = pattern.left_most();
//     pattern.som = Some(hyperscan::SomHorizon::Large);
//     let db: StreamingDatabase = pattern.build()?;
//     Ok(db)
// }


struct CheckStats {
    num_warnings: usize,
    num_errors: usize,
}

fn check_rule(rule_num: usize, rule: &Rule) -> Result<CheckStats> {
    let _span = error_span!("rule", "{}:{}", rule_num + 1, rule.name).entered();

    let mut num_warnings = 0;
    let mut num_errors = 0;

    let num_examples = rule.examples.len();
    if num_examples == 0 {
        warn!("Rule has no examples");
        num_warnings += 1;
    }

    match rule.as_regex() {
        Err(e) => {
            error!("Regex: failed to compile pattern: {}", e);
            num_errors += 1;
        }
        Ok(pat) => {
            let mut num_succeeded = 0;
            let mut num_failed = 0;

            for (example_num, example) in rule.examples.iter().enumerate() {
                if pat.find(example.as_bytes()).is_none() {
                    error!("Regex: failed to match example {}", example_num);
                    num_failed += 1;
                    num_errors += 1;
                } else {
                    num_succeeded += 1;
                }
            }

            let num_total = num_succeeded + num_failed;
            if num_total > 0 {
                debug!("Regex: {}/{} examples succeeded", num_succeeded, num_total);
            }
        }
    };

    // match hs_compile_pattern_streaming(&rule.pattern) {
    //     Err(e) => {
    //         error!("Hyperscan: failed to compile streaming pattern: {}", e);
    //         num_errors += 1;
    //     }
    //     Ok(_db) => {}
    // }

    match hs_compile_pattern(&rule.uncommented_pattern()) {
        Err(e) => {
            error!("Hyperscan: failed to compile pattern: {}", e);
            num_errors += 1;
        }
        Ok(db) => {
            let scratch = db.alloc_scratch()?;
            let mut num_succeeded = 0;
            let mut num_failed = 0;

            for (example_num, example) in rule.examples.iter().enumerate() {
                let mut matched = false;
                db.scan(example, &scratch, |_id, _from, _to, _flags| {
                    matched = true;
                    Matching::Continue
                })?;
                if !matched {
                    error!("Hyperscan: failed to match example {}", example_num);
                    num_failed += 1;
                    num_errors += 1;
                } else {
                    num_succeeded += 1;
                }
            }

            let num_total = num_succeeded + num_failed;
            if num_total > 0 {
                debug!("Hyperscan: {}/{} examples succeeded", num_succeeded, num_total);
            }
        }
    }

    if num_warnings == 0 && num_errors == 0 {
        debug!("No issues detected");
    } else {
        debug!("{} errors and {} warnings", num_errors, num_warnings);
    }

    Ok(CheckStats {
        num_warnings,
        num_errors,
    })
}
