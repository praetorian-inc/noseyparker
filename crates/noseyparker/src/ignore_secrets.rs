//! Support for ignoring secrets by value.
//!
//! This module provides the `IgnoreSecrets` type, which maintains a set of secret
//! values that should be ignored during scanning. This is useful for filtering out
//! known false positives like AWS example keys.

use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A set of secret values to ignore during scanning.
///
/// Secret values are matched by exact byte comparison against capture groups
/// from rule matches.
#[derive(Debug, Default)]
pub struct IgnoreSecrets {
    secrets: HashSet<Vec<u8>>,
}

impl IgnoreSecrets {
    /// Create a new empty `IgnoreSecrets` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load secrets from a file.
    ///
    /// The file format is one secret value per line. Lines starting with `#`
    /// are treated as comments and ignored. Empty lines are also ignored.
    pub fn load_from_file(&mut self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            self.add_line(&line);
        }
        Ok(())
    }

    /// Load secrets from a string.
    ///
    /// The format is the same as for `load_from_file`.
    pub fn load_from_str(&mut self, content: &str) {
        for line in content.lines() {
            self.add_line(line);
        }
    }

    /// Add a single line, handling comments and whitespace.
    fn add_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            self.secrets.insert(trimmed.as_bytes().to_vec());
        }
    }

    /// Check if a secret value should be ignored.
    pub fn should_ignore(&self, secret: &[u8]) -> bool {
        self.secrets.contains(secret)
    }

    /// Check if any of the given byte slices matches an ignored secret.
    pub fn any_ignored(&self, values: &[&[u8]]) -> bool {
        values.iter().any(|v| self.should_ignore(v))
    }

    /// Return the number of ignored secrets loaded.
    pub fn len(&self) -> usize {
        self.secrets.len()
    }

    /// Return true if no secrets are loaded.
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let ignore = IgnoreSecrets::new();
        assert!(ignore.is_empty());
        assert_eq!(ignore.len(), 0);
        assert!(!ignore.should_ignore(b"anything"));
    }

    #[test]
    fn test_load_from_str() {
        let mut ignore = IgnoreSecrets::new();
        ignore.load_from_str(
            r#"
# This is a comment
AKIAIOSFODNN7EXAMPLE
wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

  # Another comment with leading space
  SPACED_SECRET  
"#,
        );

        assert_eq!(ignore.len(), 3);
        assert!(ignore.should_ignore(b"AKIAIOSFODNN7EXAMPLE"));
        assert!(ignore.should_ignore(b"wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"));
        assert!(ignore.should_ignore(b"SPACED_SECRET"));
        assert!(!ignore.should_ignore(b"NOT_IN_LIST"));
    }

    #[test]
    fn test_any_ignored() {
        let mut ignore = IgnoreSecrets::new();
        ignore.load_from_str("SECRET1\nSECRET2");

        assert!(ignore.any_ignored(&[b"SECRET1"]));
        assert!(ignore.any_ignored(&[b"OTHER", b"SECRET2"]));
        assert!(!ignore.any_ignored(&[b"OTHER1", b"OTHER2"]));
        assert!(!ignore.any_ignored(&[]));
    }

    #[test]
    fn test_comments_and_empty_lines() {
        let mut ignore = IgnoreSecrets::new();
        ignore.load_from_str(
            r#"
# Comment at start
  # Comment with leading whitespace
VALUE1
  
VALUE2
# Comment in middle

VALUE3
"#,
        );

        assert_eq!(ignore.len(), 3);
        assert!(!ignore.should_ignore(b"# Comment at start"));
        assert!(!ignore.should_ignore(b""));
    }
}
