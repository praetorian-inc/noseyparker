use anyhow::{bail, Context, Result};
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::{debug, debug_span};

// -------------------------------------------------------------------------------------------------
// Rule
// -------------------------------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A pattern-based rule as represented syntactically.
pub struct Rule {
    /// The name of the rule
    pub name: String,

    /// The regex pattern that the rule uses
    pub pattern: String,

    /// Example inputs that this rule is expected to match
    #[serde(default)]
    pub examples: Vec<String>,

    /// Example inputs that this rule is expected _not_ to match
    #[serde(default)]
    pub negative_examples: Vec<String>,

    /// Freeform references for the rule; usually URLs
    #[serde(default)]
    pub references: Vec<String>,
}

lazy_static! {
    // used to strip out hyperscan-style comments like `(?# this is a comment)`,
    // which Rust's regex crate doesn't like
    static ref RULE_COMMENTS_PATTERN: Regex = Regex::new(r"\(\?#[^)]*\)")
        .expect("comment-stripping regex should compile");
}

impl Rule {
    pub fn uncommented_pattern(&self) -> Cow<'_, str> {
        RULE_COMMENTS_PATTERN.replace_all(&self.pattern, "")
    }

    const REGEX_SIZE_LIMIT: usize = 16 * 1024 * 1024;

    fn build_regex(pattern: &str) -> Result<regex::bytes::Regex> {
        let pattern = regex::bytes::RegexBuilder::new(pattern)
            .unicode(false)
            .size_limit(Self::REGEX_SIZE_LIMIT)
            .build()?;
        Ok(pattern)
    }

    pub fn as_regex(&self) -> Result<regex::bytes::Regex> {
        Self::build_regex(&self.uncommented_pattern())
    }

    /// Compile this rule into a regex with an end-of-line anchor appended.
    ///
    /// Examples:
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// # use noseyparker::rules::Rule;
    /// let r = Rule {
    ///     name: "Test rule".to_string(),
    ///     pattern: r"hello\s*world".to_string(),
    ///     examples: vec![],
    ///     negative_examples: vec![],
    ///     references: vec![],
    /// };
    /// assert_eq!(r.as_anchored_regex().unwrap().as_str(), r"hello\s*world$");
    /// ```
    pub fn as_anchored_regex(&self) -> Result<regex::bytes::Regex> {
        Self::build_regex(&format!("{}$", self.uncommented_pattern()))
    }
}

// -------------------------------------------------------------------------------------------------
// Rules
// -------------------------------------------------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {
    pub fn from_default_rules() -> Result<Self> {
        use crate::defaults::DEFAULT_RULES_DIR;
        let mut yaml_files: Vec<(&'_ Path, &'_ [u8])> = DEFAULT_RULES_DIR
            .find("**/*.yml")
            .expect("Constant glob should compile")
            .filter_map(|e| e.as_file())
            .map(|f| (f.path(), f.contents()))
            .collect();
        yaml_files.sort_by_key(|t| t.0);

        let mut rules = Rules { rules: Vec::new() };
        for &(path, contents) in yaml_files.iter() {
            let rs: Self = serde_yaml::from_reader(contents)
                .with_context(|| format!("Failed to load YAML from {}", path.display()))?;
            rules.extend(rs);
        }

        Ok(rules)
    }

    pub fn new() -> Self {
        Rules { rules: Vec::new() }
    }

    pub fn from_paths<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        let mut rules = Rules::new();
        for input in paths {
            let input = input.as_ref();
            if input.is_file() {
                let new_rules = Rules::from_yaml_file(input)?;
                rules.extend(new_rules);
            } else if input.is_dir() {
                let new_rules = Rules::from_directory(input)?;
                rules.extend(new_rules);
            } else {
                bail!("Unhandled input type: {} is neither a file nor directory", input.display());
            }
        }
        debug!("Loaded {} rules from {} paths", rules.len(), paths.len());
        Ok(rules)
    }

    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let _span = debug_span!("Rules::from_yaml_file", "{}", path.display()).entered();
        let infile =
            File::open(path).with_context(|| format!("Failed to read rules from {}", path.display()))?;
        let reader = BufReader::new(infile);
        let rules: Self = serde_yaml::from_reader(reader)
            .with_context(|| format!("Failed to load YAML from {}", path.display()))?;
        debug!("Loaded {} rules from {}", rules.len(), path.display());
        Ok(rules)
    }

    pub fn from_yaml_files<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        let mut rules = Vec::new();
        for path in paths {
            let file_rules = Rules::from_yaml_file(path.as_ref())?;
            rules.extend(file_rules);
        }
        debug!("Loaded {} rules from {} files", rules.len(), paths.len());
        Ok(Rules { rules })
    }

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

    #[inline]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self::new()
    }
}

impl Extend<Rule> for Rules {
    fn extend<T: IntoIterator<Item = Rule>>(&mut self, iter: T) {
        self.rules.extend(iter);
    }
}

impl IntoIterator for Rules {
    type Item = Rule;
    type IntoIter = <Vec<Rule> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.rules.into_iter()
    }
}

// -------------------------------------------------------------------------------------------------
// test
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    // use proptest::string::string_regex;

    proptest! {
        // Idea: load up psst rules, and for each one, generate strings conforming to its pattern, then
        // check some properties.
        //
        // See https://altsysrq.github.io/proptest-book/proptest/tutorial/transforming-strategies.html
        #[test]
        fn regex_gen_noop(s in r"((?:A3T[A-Z0-9]|AKIA|AGPA|AIDA|AROA|AIPA|ANPA|ANVA|ASIA)[A-Z0-9]{16})") {
            println!("{}", s);
        }
    }

    #[test]
    #[should_panic]
    fn failure() {
        assert_eq!(5, 42);
    }
}
