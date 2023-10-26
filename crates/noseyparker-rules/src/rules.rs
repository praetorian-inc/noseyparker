use anyhow::{bail, Context, Result};
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::{debug, debug_span};

use crate::Rule;

// -------------------------------------------------------------------------------------------------
// Rules
// -------------------------------------------------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {
    pub fn from_paths_and_contents<'a, I: IntoIterator<Item=(&'a Path, &'a [u8])>>(iterable: I) -> Result<Self> {
        let mut rules = Rules { rules: Vec::new() };
        for (path, contents) in iterable.into_iter() {
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
                rules.extend(Rules::from_yaml_file(input)?);
            } else if input.is_dir() {
                rules.extend(Rules::from_directory(input)?);
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
