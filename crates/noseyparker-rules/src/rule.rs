use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use noseyparker_digest::sha1_hexdigest;

/// A pattern-based rule as represented syntactically.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct RuleSyntax {
    /// The human-readable name of the rule
    pub name: String,

    /// A globally-unique identifier for the rule
    pub id: String,

    /// The regex pattern that the rule uses
    pub pattern: String,

    /// A human-readable description of the rule, often answering what was found and how an attacker could use it
    #[serde(default)]
    pub description: Option<String>,

    /// Example inputs that this rule is expected to match
    #[serde(default)]
    pub examples: Vec<String>,

    /// Example inputs that this rule is expected _not_ to match
    #[serde(default)]
    pub negative_examples: Vec<String>,

    /// Freeform references for the rule; usually URLs
    #[serde(default)]
    pub references: Vec<String>,

    /// A list of string categories for the rule
    #[serde(default)]
    pub categories: Vec<String>,
}

lazy_static! {
    // used to strip out vectorscan-style comments like `(?# this is a comment)`,
    // which Rust's regex crate doesn't like
    static ref RULE_COMMENTS_PATTERN: Regex = Regex::new(r"\(\?#[^)]*\)")
        .expect("comment-stripping regex should compile");
}

impl RuleSyntax {
    /// Get the pattern for this rule with any comments removed.
    pub fn uncommented_pattern(&self) -> Cow<'_, str> {
        RULE_COMMENTS_PATTERN.replace_all(&self.pattern, "")
    }

    // NOTE: Some of the patterns from default rules are complicated patterns that require more
    // than the default regex size limit to compile. 16MiB has been enough so far...
    const REGEX_SIZE_LIMIT: usize = 16 * 1024 * 1024;

    fn build_regex(pattern: &str) -> Result<regex::bytes::Regex> {
        let pattern = regex::bytes::RegexBuilder::new(pattern)
            .unicode(false)
            .size_limit(Self::REGEX_SIZE_LIMIT)
            .build()?;
        Ok(pattern)
    }

    /// Compile this pattern into a regular expression.
    pub fn as_regex(&self) -> Result<regex::bytes::Regex> {
        Self::build_regex(&self.uncommented_pattern())
    }

    /// Compile this rule into a regex with an end-of-line anchor appended.
    /// This will ensure that any matches of this rule occur at the end of input.
    ///
    /// Examples:
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// # use noseyparker_rules::RuleSyntax;
    /// let r = RuleSyntax {
    ///     name: "Test rule".to_string(),
    ///     id: "test.1".to_string(),
    ///     pattern: r"hello\s*world".to_string(),
    ///     description: None,
    ///     examples: vec![],
    ///     negative_examples: vec![],
    ///     references: vec![],
    ///     categories: vec![],
    /// };
    /// assert_eq!(r.as_anchored_regex().unwrap().as_str(), r"hello\s*world\z");
    /// ```
    pub fn as_anchored_regex(&self) -> Result<regex::bytes::Regex> {
        Self::build_regex(&format!(r"{}\z", self.uncommented_pattern()))
    }

    /// Compute the content-based structural ID of this rule.
    pub fn structural_id(&self) -> String {
        sha1_hexdigest(self.pattern.as_bytes())
    }

    /// Return a JSON serialization of this rule.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("should be able to serialize rule syntax as JSON")
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    syntax: RuleSyntax,
    structural_id: String,
}

impl Rule {
    pub fn new(syntax: RuleSyntax) -> Self {
        Self {
            structural_id: syntax.structural_id(),
            syntax,
        }
    }

    // Get the AST of this rule.
    pub fn syntax(&self) -> &RuleSyntax {
        &self.syntax
    }

    pub fn json_syntax(&self) -> String {
        self.syntax.to_json()
    }

    pub fn structural_id(&self) -> &str {
        &self.structural_id
    }

    pub fn name(&self) -> &str {
        &self.syntax.name
    }

    pub fn id(&self) -> &str {
        &self.syntax.id
    }
}
