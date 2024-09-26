use schemars::JsonSchema;
use serde::ser::SerializeSeq;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::provenance::Provenance;

// XXX this could be reworked to use https://docs.rs/nonempty instead of handrolling that

/// A non-empty set of `Provenance` entries.
#[derive(Debug)]
pub struct ProvenanceSet {
    provenance: Provenance,
    more_provenance: Vec<Provenance>,
}

/// Serialize `ProvenanceSet` as a flat sequence
impl serde::Serialize for ProvenanceSet {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.len()))?;
        for p in self.iter() {
            seq.serialize_element(p)?;
        }
        seq.end()
    }
}

impl JsonSchema for ProvenanceSet {
    fn schema_name() -> String {
        "ProvenanceSet".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let s = <Vec<Provenance>>::json_schema(gen);
        let mut o = s.into_object();
        o.array().min_items = Some(1);
        let md = o.metadata();
        md.description = Some("A non-empty set of `Provenance` entries".into());
        schemars::schema::Schema::Object(o)
    }
}

impl ProvenanceSet {
    #[inline]
    pub fn single(provenance: Provenance) -> Self {
        Self {
            provenance,
            more_provenance: vec![],
        }
    }

    /// Create a new `ProvenanceSet` from the given items, filtering out redundant less-specific
    /// `Provenance` records.
    pub fn new(provenance: Provenance, more_provenance: Vec<Provenance>) -> Self {
        let mut git_repos_with_detailed: HashSet<Arc<PathBuf>> = HashSet::new();

        for p in std::iter::once(&provenance).chain(&more_provenance) {
            if let Provenance::GitRepo(e) = p {
                if e.first_commit.is_some() {
                    git_repos_with_detailed.insert(e.repo_path.clone());
                }
            }
        }

        let mut it = std::iter::once(provenance)
            .chain(more_provenance)
            .filter(|p| match p {
                Provenance::GitRepo(e) => {
                    e.first_commit.is_some() || !git_repos_with_detailed.contains(&e.repo_path)
                }
                Provenance::File(_) => true,
                Provenance::Extended(_) => true,
            });

        Self {
            provenance: it.next().unwrap(),
            more_provenance: it.collect(),
        }
    }

    #[inline]
    pub fn try_from_iter<I>(it: I) -> Option<Self>
    where
        I: IntoIterator<Item = Provenance>,
    {
        let mut it = it.into_iter();
        let provenance = it.next()?;
        let more_provenance = it.collect();
        Some(Self::new(provenance, more_provenance))
    }

    #[inline]
    pub fn first(&self) -> &Provenance {
        &self.provenance
    }

    #[allow(clippy::len_without_is_empty)]
    #[inline]
    pub fn len(&self) -> usize {
        1 + self.more_provenance.len()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Provenance> {
        std::iter::once(&self.provenance).chain(&self.more_provenance)
    }
}

impl IntoIterator for ProvenanceSet {
    type Item = Provenance;
    type IntoIter =
        std::iter::Chain<std::iter::Once<Provenance>, <Vec<Provenance> as IntoIterator>::IntoIter>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.provenance).chain(self.more_provenance)
    }
}

impl From<Provenance> for ProvenanceSet {
    fn from(p: Provenance) -> Self {
        Self::single(p)
    }
}
