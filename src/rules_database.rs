use anyhow::{bail, Context, Result};
use hyperscan::prelude::{Builder, Pattern, Patterns};
use regex::bytes::Regex;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, debug_span};

use crate::rules::{Rule, Rules};

pub struct RulesDatabase {
    // NOTE: pub(crate) here so that `Matcher` can access these
    pub(crate) rules: Rules,
    pub(crate) anchored_regexes: Vec<Regex>,
    pub(crate) hsdb: hyperscan::BlockDatabase,
}

impl RulesDatabase {
    /// Create a new `RulesDatabase` with the built-in default set of rules.
    pub fn from_default_rules() -> Result<Self> {
        Self::from_rules(Rules::from_default_rules()?)
    }

    /// Create a new `RulesDatabase` from rules files found within the given directory.
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_rules(Rules::from_directory(path)?)
    }

    /// Create a new `RulesDatabase` from the given set of rules.
    pub fn from_rules(rules: Rules) -> Result<Self> {
        let _span = debug_span!("RulesDatabase::from_rules").entered();

        if rules.rules.is_empty() {
            bail!("No rules to compile");
        }

        let patterns = rules
            .rules
            .iter()
            .map(|r| {
                Pattern::new(&r.pattern)
                    .with_context(|| format!("Failed to create rule for pattern {}", &r.pattern))
            })
            .collect::<Result<Vec<Pattern>>>()?;

        let t1 = Instant::now();
        let hsdb = Patterns::build(&Patterns::from(patterns))?;
        let d1 = t1.elapsed().as_secs_f64();

        let t2 = Instant::now();
        let anchored_regexes = rules
            .rules
            .iter()
            .map(Rule::as_anchored_regex)
            .collect::<Result<Vec<Regex>>>()?;
        let d2 = t2.elapsed().as_secs_f64();

        debug!("Compiled {} rules: hyperscan {}s; regex {}s", rules.rules.len(), d1, d2);
        Ok(RulesDatabase {
            rules,
            hsdb,
            anchored_regexes,
        })
    }

    // pub fn serialize(&self) -> Result<()> {
    //     let bytes = self.hsdb.serialize()?;
    //     debug!("{} bytes for serialized database", bytes.len());
    //     panic!("unimplemented!");
    //     // Ok(())
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    pub fn test_hyperscan_sanity() -> Result<()> {
        use hyperscan::prelude::*;

        let input = "some test data for hyperscan";
        let pattern = pattern! {"test"; CASELESS | SOM_LEFTMOST};
        let db: BlockDatabase = pattern.build()?;
        let scratch = db.alloc_scratch()?;
        let mut matches = vec![];

        db.scan(input, &scratch, |id, from, to, _flags| {
            println!("found pattern #{} @ [{}, {})", id, from, to);
            matches.push(from..to);
            Matching::Continue
        })?;

        assert_eq!(matches, vec![5..9]);
        Ok(())
    }
}
