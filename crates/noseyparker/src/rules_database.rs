use anyhow::{bail, Result};
use vectorscan::{BlockDatabase, Pattern, Flag};
use regex::bytes::Regex;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, debug_span};

use crate::rules::{Rule, Rules};

pub struct RulesDatabase {
    // NOTE: pub(crate) here so that `Matcher` can access these
    pub(crate) rules: Rules,
    pub(crate) anchored_regexes: Vec<Regex>,
    pub(crate) vsdb: BlockDatabase,
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
            .enumerate()
            .map(|(id, r)| {
                let id = id.try_into().unwrap();
                Pattern::new(r.pattern.clone().into_bytes(), Flag::default(), Some(id))
            })
            .collect::<Vec<Pattern>>();

        let t1 = Instant::now();
        let vsdb = BlockDatabase::new(patterns)?;
        let d1 = t1.elapsed().as_secs_f64();

        let t2 = Instant::now();
        let anchored_regexes = rules
            .rules
            .iter()
            .map(Rule::as_anchored_regex)
            .collect::<Result<Vec<Regex>>>()?;
        let d2 = t2.elapsed().as_secs_f64();

        debug!("Compiled {} rules: vectorscan {}s; regex {}s", rules.rules.len(), d1, d2);
        Ok(RulesDatabase {
            rules,
            vsdb,
            anchored_regexes,
        })
    }

    // pub fn serialize(&self) -> Result<()> {
    //     let bytes = self.vsdb.serialize()?;
    //     debug!("{} bytes for serialized database", bytes.len());
    //     panic!("unimplemented!");
    //     // Ok(())
    // }

    pub fn num_rules(&self) -> usize {
        self.rules.len()
    }

    pub fn get_rule(&self, index: usize) -> Option<&Rule> {
        self.rules.rules.get(index)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    pub fn test_vectorscan_sanity() -> Result<()> {
        use vectorscan::{BlockDatabase, Pattern, Scan, BlockScanner};

        let input = b"some test data for vectorscan";
        let pattern = Pattern::new(b"test".to_vec(), Flag::CASELESS | Flag::SOM_LEFTMOST, None);
        let db: BlockDatabase = BlockDatabase::new(vec![pattern])?;

        let mut scanner = BlockScanner::new(&db)?;

        let mut matches: Vec<(u64, u64)> = vec![];
        scanner.scan(input, |id: u32, from: u64, to: u64, _flags: u32| {
            println!("found pattern #{} @ [{}, {})", id, from, to);
            matches.push((from, to));
            Scan::Continue
        })?;

        assert_eq!(matches, vec![(5, 9)]);
        Ok(())
    }
}
