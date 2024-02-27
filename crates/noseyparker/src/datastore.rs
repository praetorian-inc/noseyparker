use anyhow::{bail, Context, Result};
use bstr::BString;
use indoc::indoc;
use noseyparker_rules::Rule;
use rusqlite::Connection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span};

use crate::blob_metadata::BlobMetadata;
use crate::git_url::GitUrl;
use crate::location::{Location, OffsetSpan, SourcePoint, SourceSpan};
use crate::match_type::{Groups, Match};
use crate::provenance::Provenance;
use crate::provenance_set::ProvenanceSet;
use crate::snippet::Snippet;

const SCHEMA_60: &str = include_str!("datastore/schema_60.sql");

// -------------------------------------------------------------------------------------------------
// Datastore
// -------------------------------------------------------------------------------------------------

/// The source of truth for Nosey Parker findings and runtime state.
///
/// A `Datastore` resides on disk as a directory, and stores a number of things:
///
/// - A sqlite database for recording findings and scan information
/// - A scratch directory for providing temporary directories and files
/// - A directory used for storing clones of Git repositories
///
/// Note that a `Datastore` is not `Sync`, and thus cannot be directly shared between threads.
/// The recommended pattern in a case that requires concurrent access is to have a single thread
/// that mediates access to the `Datastore`.
///
/// Accessing a single `Datastore` from multiple processes is untested and may not work correctly.
/// This implementation has not built-in mechanism to check for or prevent multi-process access.
pub struct Datastore {
    /// The root directory of everything contained in this `Datastore`.
    root_dir: PathBuf,

    /// A connection to the database backing this `Datastore`.
    conn: Connection,
}

// Public implementation
impl Datastore {
    /// Create a new datastore at `root_dir` if one does not exist,
    /// or open an existing one if present.
    pub fn create_or_open(root_dir: &Path, cache_size: i64) -> Result<Self> {
        debug!("Attempting to create or open an existing datastore at {}", root_dir.display());

        Self::create(root_dir, cache_size).or_else(|e| {
            debug!(
                "Failed to create datastore: {e:#}: will try to open existing datastore instead"
            );
            Self::open(root_dir, cache_size)
        })
    }

    /// Open the existing datastore at `root_dir`.
    pub fn open(root_dir: &Path, cache_size: i64) -> Result<Self> {
        debug!("Attempting to open existing datastore at {}", root_dir.display());

        let ds = Self::open_impl(root_dir, cache_size)?;
        ds.check_schema_version()?;

        let scratch_dir = ds.scratch_dir();
        std::fs::create_dir_all(&scratch_dir).with_context(|| {
            format!("Failed to create scratch directory {}", scratch_dir.display(),)
        })?;

        let clones_dir = ds.clones_dir();
        std::fs::create_dir_all(&clones_dir).with_context(|| {
            format!("Failed to create clones directory {}", clones_dir.display(),)
        })?;

        let blobs_dir = ds.blobs_dir();
        std::fs::create_dir_all(&blobs_dir).with_context(|| {
            format!("Failed to create blobs directory {}", blobs_dir.display(),)
        })?;

        Ok(ds)
    }

    /// Create a new datastore at `root_dir` and open it.
    pub fn create(root_dir: &Path, cache_size: i64) -> Result<Self> {
        debug!("Attempting to create new datastore at {}", root_dir.display());

        // Create datastore directory
        std::fs::create_dir(root_dir).with_context(|| {
            format!("Failed to create datastore root directory at {}", root_dir.display())
        })?;

        // Generate .gitignore file
        std::fs::write(root_dir.join(".gitignore"), "*\n").with_context(|| {
            format!("Failed to write .gitignore to datastore at {}", root_dir.display())
        })?;

        let mut ds = Self::open_impl(root_dir, cache_size)?;

        ds.migrate_0_60()
            .context("Failed to initialize database schema")?;

        Self::open(root_dir, cache_size)
    }

    /// Get the path to this datastore's scratch directory.
    pub fn scratch_dir(&self) -> PathBuf {
        self.root_dir.join("scratch")
    }

    /// Get the path to this datastore's clones directory.
    pub fn clones_dir(&self) -> PathBuf {
        self.root_dir.join("clones")
    }

    /// Get the path to this datastore's blobs directory.
    pub fn blobs_dir(&self) -> PathBuf {
        self.root_dir.join("blobs")
    }

    /// Get the root directory that contains this `Datastore`.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Get a path for a local clone of the given git URL within this datastore's clones directory.
    pub fn clone_destination(&self, repo: &GitUrl) -> Result<std::path::PathBuf> {
        clone_destination(&self.clones_dir(), repo)
    }

    /// Analyze the datastore's sqlite database, potentially allowing for better query planning
    pub fn analyze(&self) -> Result<()> {
        let _span = debug_span!("Datastore::analyze", "{}", self.root_dir.display()).entered();
        self.conn.execute("analyze", [])?;
        // self.conn.execute("pragma wal_checkpoint(truncate)", [])?;
        Ok(())
    }
}

/// A datastore-specific ID of a blob; simply a newtype-like wrapper around an i64.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct BlobIdInt(i64);

/// A datastore-specific ID of a rule; simply a newtype-like wrapper around an i64.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct RuleIdInt(i64);

/// A datastore-specific ID of a snippet; simply a newtype-like wrapper around an i64.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SnippetIdInt(i64);

/// A datastore-specific ID of a match; simply a newtype-like wrapper around an i64.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MatchId(i64);

type BatchEntry = (ProvenanceSet, BlobMetadata, Vec<(Option<f64>, Match)>);

pub struct Transaction<'a> {
    inner: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    /// Commit this `Transaction`, consuming it.
    pub fn commit(self) -> Result<()> {
        self.inner.commit()?;
        Ok(())
    }

    fn mk_record_rule(&'a self) -> Result<impl FnMut(&'a Rule) -> rusqlite::Result<RuleIdInt>> {
        let mut get_id = self.inner.prepare_cached(indoc! {r#"
            select id from rule
            where structural_id = ? and name = ? and text_id = ? and syntax = ?
        "#})?;

        let mut set_id = self.inner.prepare_cached(indoc! {r#"
            insert into rule(structural_id, name, text_id, syntax)
            values (?, ?, ?, ?)
            on conflict do update set syntax = excluded.syntax
            where syntax != excluded.syntax
            returning id
        "#})?;

        let f = move |r: &Rule| -> rusqlite::Result<RuleIdInt> {
            let json_syntax = r.json_syntax();
            let rule_id = add_if_missing_simple(
                &mut get_id,
                &mut set_id,
                val_from_row,
                (r.structural_id(), r.name(), r.id(), &json_syntax),
            )?;
            Ok(RuleIdInt(rule_id))
        };

        Ok(f)
    }

    pub fn record_rules(&self, rules: &[Rule]) -> Result<()> {
        let mut record_rule = self.mk_record_rule()?;
        for rule in rules {
            record_rule(rule)?;
        }
        Ok(())
    }

    /// Return a closure that records a blob's metadata (only if necessary), returning its integer ID
    fn mk_record_blob_metadata(
        &'a self,
    ) -> Result<impl FnMut(&'a BlobMetadata) -> rusqlite::Result<BlobIdInt>> {
        let mut get_id = self.inner.prepare_cached(indoc! {r#"
            select id from blob where blob_id = ? and size = ?
        "#})?;

        let mut set_id = self.inner.prepare_cached(indoc! {r#"
            insert into blob(blob_id, size)
            values (?, ?)
            returning id
        "#})?;

        let mut set_mime_essence = self.inner.prepare_cached(indoc! {r#"
            insert or ignore into blob_mime_essence(blob_id, mime_essence)
            values (?, ?)
        "#})?;

        let mut set_charset = self.inner.prepare_cached(indoc! {r#"
            insert or ignore into blob_charset(blob_id, charset)
            values (?, ?)
        "#})?;

        let f = move |b: &BlobMetadata| -> rusqlite::Result<BlobIdInt> {
            let blob_id = add_if_missing_simple(
                &mut get_id,
                &mut set_id,
                val_from_row,
                (&b.id, b.num_bytes),
            )?;

            if let Some(mime_essence) = b.mime_essence() {
                set_mime_essence.execute((blob_id, mime_essence))?;
            }

            if let Some(charset) = b.charset() {
                set_charset.execute((blob_id, charset))?;
            }

            Ok(BlobIdInt(blob_id))
        };

        Ok(f)
    }

    /// Record provenance metadata for a blob given its integer ID
    fn mk_record_provenance(
        &'a self,
    ) -> Result<impl FnMut(BlobIdInt, &'a Provenance) -> rusqlite::Result<()>> {
        let mut add_provenance = self.inner.prepare_cached(indoc! {r#"
            insert into blob_provenance(blob_id, provenance)
            values (?, ?)
            on conflict do nothing
        "#})?;

        let f = move |BlobIdInt(blob_id), provenance| -> rusqlite::Result<()> {
            add_provenance.execute((blob_id, provenance))?;
            Ok(())
        };

        Ok(f)
    }

    /// Record a contextual snippet, returning an integer ID for it
    fn mk_record_snippet(&'a self) -> Result<impl FnMut(&'a [u8]) -> rusqlite::Result<i64>> {
        let mut get = self.inner.prepare_cached(indoc! {r#"
            select id from snippet where snippet = ?
        "#})?;

        let mut set = self.inner.prepare_cached(indoc! {r#"
            insert into snippet(snippet)
            values (?)
            returning id
        "#})?;

        Ok(move |blob| add_if_missing_simple(&mut get, &mut set, val_from_row, (blob,)))
    }

    /// Record a match, returning whether it was new or not
    fn mk_record_match(
        &'a self,
    ) -> Result<impl FnMut(BlobIdInt, &'a Match, &'a Option<f64>) -> rusqlite::Result<bool>> {
        let mut record_snippet = self.mk_record_snippet()?;

        let mut get_finding_id = self.inner.prepare_cached(indoc! {r#"
            select f.id
            from finding f
            where f.finding_id = ?
        "#})?;

        let mut set_finding_id = self.inner.prepare_cached(indoc! {r#"
            insert into finding (finding_id, rule_id, groups)
            select ?1, r.id, ?3
            from rule r
            where r.structural_id = ?2
            returning id
        "#})?;

        let mut get_match_id = self.inner.prepare_cached(indoc! {r#"
            select m.id
            from
                match m
            where
                m.blob_id = ?
                and m.start_byte = ?
                and m.end_byte = ?
                and m.finding_id = ?
        "#})?;

        let mut update_match = self.inner.prepare_cached(indoc! {r#"
            update match
            set
                structural_id = ?2,
                finding_id = ?3,
                before_snippet_id = ?4,
                matching_snippet_id = ?5,
                after_snippet_id = ?6
            where
                id = ?1 and
                (
                    structural_id != ?2 or
                    finding_id != ?3 or
                    before_snippet_id != ?4 or
                    matching_snippet_id != ?5 or
                    after_snippet_id != ?6
                )
        "#})?;

        let mut add_match = self.inner.prepare_cached(indoc! {r#"
            insert into match (
                structural_id,
                finding_id,
                blob_id,
                start_byte,
                end_byte,
                before_snippet_id,
                matching_snippet_id,
                after_snippet_id
            )
            select ?, ?, ?, ?, ?, ?, ?, ?
            returning id
        "#})?;

        let mut add_blob_source_span = self.inner.prepare_cached(indoc! {r#"
            insert into blob_source_span (blob_id, start_byte, end_byte, start_line, start_column, end_line, end_column)
            values (?, ?, ?, ?, ?, ?, ?)
            on conflict do update set
                start_line = excluded.start_line,
                start_column = excluded.start_column,
                end_line = excluded.end_line,
                end_column = excluded.end_column
            where
                start_line != excluded.start_line
                or start_column != excluded.start_column
                or end_line != excluded.end_line
                or end_column != excluded.end_column
        "#})?;

        let mut set_score = self.inner.prepare_cached(indoc! {r#"
            insert into match_score (match_id, score)
            values (?, ?)
            on conflict do update set score = excluded.score
        "#})?;

        let f = move |BlobIdInt(blob_id), m: &'a Match, score: &'a Option<f64>| {
            let start_byte = m.location.offset_span.start;
            let end_byte = m.location.offset_span.end;
            let rule_structural_id = &m.rule_structural_id;
            let structural_id = &m.structural_id;
            let finding_id = &m.finding_id();
            let groups = &m.groups;
            let source_span = &m.location.source_span;

            add_blob_source_span.execute((
                blob_id,
                start_byte,
                end_byte,
                source_span.start.line,
                source_span.start.column,
                source_span.end.line,
                source_span.end.column,
            ))?;

            let finding_id: i64 = {
                match get_finding_id
                    .query_map((finding_id,), val_from_row)?
                    .next()
                {
                    Some(finding_id) => finding_id?,
                    None => set_finding_id
                        .query_row((finding_id, rule_structural_id, groups), val_from_row)?,
                }
            };

            let snippet = &m.snippet;
            let before_snippet_id = record_snippet(snippet.before.as_slice())?;
            let matching_snippet_id = record_snippet(snippet.matching.as_slice())?;
            let after_snippet_id = record_snippet(snippet.after.as_slice())?;

            let (match_id, new) = if let Some(match_id) = get_match_id
                .query_map((blob_id, start_byte, end_byte, finding_id), val_from_row)?
                .next()
            {
                let match_id: i64 = match_id?;
                // existing match; update if needed
                update_match.execute((
                    match_id,
                    structural_id,
                    finding_id,
                    before_snippet_id,
                    matching_snippet_id,
                    after_snippet_id,
                ))?;
                (match_id, false)
            } else {
                // totally new match
                let match_id = add_match.query_row(
                    (
                        structural_id,
                        finding_id,
                        blob_id,
                        start_byte,
                        end_byte,
                        before_snippet_id,
                        matching_snippet_id,
                        after_snippet_id,
                    ),
                    val_from_row,
                )?;
                (match_id, true)
            };

            if let Some(score) = score {
                set_score.execute((match_id, score))?;
            }

            Ok(new)
        };

        Ok(f)
    }

    /// Record the given data into the datastore.
    /// Returns the number of matches that were newly added.
    pub fn record(&self, batch: &[BatchEntry]) -> Result<u64> {
        let mut record_blob_metadata = self.mk_record_blob_metadata()?;
        let mut record_provenance = self.mk_record_provenance()?;
        let mut record_match = self.mk_record_match()?;

        let mut num_matches_added = 0;

        for (ps, md, ms) in batch {
            // record blob metadata
            let blob_id = record_blob_metadata(md).context("Failed to add blob metadata")?;

            // // record provenance metadata
            for p in ps.iter() {
                record_provenance(blob_id, p).context("Failed to record blob provenance")?;
            }

            // record matches
            for (s, m) in ms {
                if record_match(blob_id, m, s).context("Failed to record match")? {
                    num_matches_added += 1;
                }
            }
        }

        Ok(num_matches_added)
    }
}

// Public implementation, recording functions
impl Datastore {
    pub fn begin(&mut self) -> Result<Transaction> {
        let inner = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        Ok(Transaction { inner })
    }
}

// Public implementation, querying functions
impl Datastore {
    pub fn get_num_matches(&self) -> Result<u64> {
        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select count(*) from match
        "#})?;
        let num_matches: u64 = stmt.query_row((), val_from_row)?;
        Ok(num_matches)
    }

    /// Summarize all recorded findings.
    pub fn summarize(&self) -> Result<FindingSummary> {
        let _span = debug_span!("Datastore::summarize", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select rule_name, total_findings, total_matches
            from finding_summary
            order by total_findings desc, rule_name, total_matches desc
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(MatchSummaryEntry {
                rule_name: row.get(0)?,
                distinct_count: row.get(1)?,
                total_count: row.get(2)?,
            })
        })?;
        let mut es = Vec::new();
        for e in entries {
            es.push(e?);
        }
        Ok(FindingSummary(es))
    }

    /// Get metadata for all groups of identical matches recorded within this `Datastore`.
    pub fn get_finding_metadata(&self) -> Result<Vec<FindingMetadata>> {
        let _span =
            debug_span!("Datastore::get_finding_metadata", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select
                finding_id,
                groups,
                rule_structural_id,
                rule_text_id,
                rule_name,
                num_matches,
                comment,
                match_statuses,
                mean_score
            from finding_denorm
            order by rule_name, rule_structural_id, mean_score desc, groups
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(FindingMetadata {
                finding_id: row.get(0)?,
                groups: row.get(1)?,
                rule_structural_id: row.get(2)?,
                rule_text_id: row.get(3)?,
                rule_name: row.get(4)?,
                num_matches: row.get(5)?,
                comment: row.get(6)?,
                statuses: row.get(7)?,
                mean_score: row.get(8)?,
            })
        })?;
        let mut es = Vec::new();
        for e in entries {
            es.push(e?);
        }
        Ok(es)
    }

    /// Get up to `limit` matches that belong to the finding with the given finding metadata.
    pub fn get_finding_data(
        &self,
        metadata: &FindingMetadata,
        limit: Option<usize>,
    ) -> Result<Vec<FindingDataEntry>> {
        let _span =
            debug_span!("Datastore::get_finding_data", "{}", self.root_dir.display()).entered();

        let match_limit: i64 = match limit {
            Some(limit) => limit.try_into().expect("limit should be convertible"),
            None => -1,
        };

        let mut get_blob_metadata_and_match = self.conn.prepare_cached(indoc! {r#"
            select
                m.blob_id,
                m.start_byte,
                m.end_byte,
                m.start_line,
                m.start_column,
                m.end_line,
                m.end_column,

                m.before_snippet,
                m.matching_snippet,
                m.after_snippet,

                m.groups,

                b.size,
                b.mime_essence,
                b.charset,

                m.id,
                m.score,
                m.comment,
                m.status,
                m.structural_id

            from match_denorm m
            inner join blob_denorm b on (m.blob_id = b.blob_id)
            where m.groups = ?1 and m.rule_structural_id = ?2
            order by m.blob_id, m.start_byte, m.end_byte
            limit ?3
        "#})?;

        let entries = get_blob_metadata_and_match.query_map(
            (&metadata.groups, &metadata.rule_structural_id, match_limit),
            |row| {
                let blob_id = row.get(0)?;
                let m = Match {
                    blob_id,
                    location: Location {
                        offset_span: OffsetSpan {
                            start: row.get(1)?,
                            end: row.get(2)?,
                        },
                        source_span: SourceSpan {
                            start: SourcePoint {
                                line: row.get(3)?,
                                column: row.get(4)?,
                            },
                            end: SourcePoint {
                                line: row.get(5)?,
                                column: row.get(6)?,
                            },
                        },
                    },
                    snippet: Snippet {
                        before: BString::new(row.get(7)?),
                        matching: BString::new(row.get(8)?),
                        after: BString::new(row.get(9)?),
                    },
                    groups: row.get(10)?,
                    rule_structural_id: metadata.rule_structural_id.clone(),
                    rule_name: metadata.rule_name.clone(),
                    rule_text_id: metadata.rule_text_id.clone(),
                    structural_id: row.get(18)?,
                };
                let num_bytes: usize = row.get(11)?;
                let mime_essence: Option<String> = row.get(12)?;
                let charset: Option<String> = row.get(13)?;
                let b = BlobMetadata {
                    id: blob_id,
                    num_bytes,
                    mime_essence,
                    charset,
                };
                let id = MatchId(row.get(14)?);
                let m_score = row.get(15)?;
                let m_comment = row.get(16)?;
                let m_status = row.get(17)?;
                Ok((b, id, m, m_score, m_comment, m_status))
            },
        )?;
        let mut es = Vec::new();
        for e in entries {
            let (md, id, m, match_score, match_comment, match_status) = e?;
            let ps = self.get_provenance_set(&md)?;
            es.push(FindingDataEntry {
                provenance: ps,
                blob_metadata: md,
                match_id: id,
                match_val: m,
                match_comment,
                match_score,
                match_status,
            });
        }
        Ok(es)
    }

    fn get_provenance_set(&self, metadata: &BlobMetadata) -> Result<ProvenanceSet> {
        let mut get = self.conn.prepare_cached(indoc! {r#"
            select provenance
            from blob_provenance_denorm
            where blob_id = ?
            order by provenance
        "#})?;

        let ps = get.query_map((metadata.id,), val_from_row)?;

        let mut results = Vec::new();
        for p in ps {
            results.push(p?);
        }
        match ProvenanceSet::try_from_iter(results) {
            Some(ps) => Ok(ps),
            None => bail!("should have at least 1 provenance entry"),
        }
    }
}

// Private implementation
impl Datastore {
    const CURRENT_SCHEMA_VERSION: u64 = 60;

    fn open_impl(root_dir: &Path, cache_size: i64) -> Result<Self> {
        let db_path = root_dir.join("datastore.db");
        let conn = Self::new_connection(&db_path, cache_size)?;
        let root_dir = root_dir.canonicalize()?;
        let ds = Self { root_dir, conn };
        Ok(ds)
    }

    fn new_connection(path: &Path, cache_size: i64) -> Result<Connection> {
        let conn = Connection::open(path)?;

        conn.pragma_update(None, "journal_mode", "wal")?; // https://www.sqlite.org/wal.html
        conn.pragma_update(None, "foreign_keys", "on")?; // https://sqlite.org/foreignkeys.html
        conn.pragma_update(None, "synchronous", "normal")?; // https://sqlite.org/pragma.html#pragma_synchronous
        conn.pragma_update(None, "cache_size", cache_size)?; // sqlite.org/pragma.html#pragma_cache_size

        Ok(conn)
    }

    fn check_schema_version(&self) -> Result<()> {
        let user_version: u64 = self
            .conn
            .pragma_query_value(None, "user_version", val_from_row)?;
        if user_version != Self::CURRENT_SCHEMA_VERSION {
            bail!(
                "Unsupported schema version {user_version} (expected {}): \
                  datastores from other versions of Nosey Parker are not supported",
                Self::CURRENT_SCHEMA_VERSION
            );
        }
        Ok(())
    }

    fn migrate_0_60(&mut self) -> Result<()> {
        let _span = debug_span!("Datastore::migrate_0_60", "{}", self.root_dir.display()).entered();
        let tx = self.conn.transaction()?;

        let get_user_version = || -> Result<u64> {
            let user_version = tx.pragma_query_value(None, "user_version", val_from_row)?;
            Ok(user_version)
        };

        let set_user_version = |user_version: u64| -> Result<()> {
            tx.pragma_update(None, "user_version", user_version)?;
            Ok(())
        };

        let user_version: u64 = get_user_version()?;
        if user_version > 0 && user_version < Self::CURRENT_SCHEMA_VERSION {
            bail!(
                "This datastore has schema version {user_version}. \
                   Datastores from other Nosey Parker versions are not supported. \
                   Rescanning the inputs with a new datastore will be required."
            );
        }
        if user_version > Self::CURRENT_SCHEMA_VERSION {
            bail!("Unknown schema version {user_version}");
        }

        if user_version == 0 {
            let new_user_version = Self::CURRENT_SCHEMA_VERSION;
            debug!("Migrating database schema from version {user_version} to {new_user_version}");
            tx.execute_batch(SCHEMA_60)?;
            set_user_version(new_user_version)?;
        }

        assert_eq!(get_user_version()?, Self::CURRENT_SCHEMA_VERSION);
        tx.commit()?;

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// Implementation Utilities
// -------------------------------------------------------------------------------------------------

/// Convert a row into a single value.
///
/// This function exists to work around an ergonomic deficiency in Rust's type system, which
/// doesn't allow defining TryFrom<&rusqlite::Row<'_>> for any T that implements rusqlite::types::FromSql.
/// Without this function, you would have to use 1-tuples all over the place instead.
fn val_from_row<T>(row: &rusqlite::Row<'_>) -> rusqlite::Result<T>
where
    T: rusqlite::types::FromSql,
{
    row.get(0)
}

/// A combinator for "upsert"-like behavior that sqlite doesn't nicely natively support.
///
/// This takes two SQL statement arguments: a getter and a setter.
/// The getter should look up 0 or 1 rows for the given parameters.
/// The setter should insert a new entry for the given parameters and return 1 row.
///
/// Any rows returned by either the getter or the setter should be convertible by the `f` function.
fn add_if_missing_simple<P, F, T>(
    get: &mut rusqlite::CachedStatement<'_>,
    set: &mut rusqlite::CachedStatement<'_>,
    f: F,
    params: P,
) -> rusqlite::Result<T>
where
    P: rusqlite::Params + Copy,
    F: Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    add_if_missing(get, set, f, params, params)
}

/// A combinator for "upsert"-like behavior that sqlite doesn't nicely natively support.
///
/// This takes two SQL statement arguments: a getter and a setter.
/// The getter should look up 0 or 1 rows for the given parameters.
/// The setter should insert a new entry for the given parameters and return 1 row.
///
/// Any rows returned by either the getter or the setter should be convertible by the `f` function.
fn add_if_missing<P1, P2, F, T>(
    get: &mut rusqlite::CachedStatement<'_>,
    set: &mut rusqlite::CachedStatement<'_>,
    f: F,
    get_params: P1,
    set_params: P2,
) -> rusqlite::Result<T>
where
    P1: rusqlite::Params,
    P2: rusqlite::Params,
    F: Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    match get.query(get_params)?.next()? {
        Some(row) => f(row),
        None => f(set
            .query(set_params)?
            .next()?
            .expect("either get or set statement must return a row")),
    }
}

/// Get a path for a local clone of the given git URL underneath `root`.
fn clone_destination(root: &std::path::Path, repo: &GitUrl) -> Result<std::path::PathBuf> {
    Ok(root.join(repo.to_path_buf()))
}

#[cfg(test)]
mod test {
    macro_rules! clone_destination_success_tests {
        ($($case_name:ident: ($root:expr, $repo:expr) => $expected:expr,)*) => {
            mod clone_destination {
                use crate::git_url::GitUrl;
                use pretty_assertions::assert_eq;
                use std::path::{PathBuf, Path};
                use std::str::FromStr;
                use super::super::clone_destination;

                $(
                    #[test]
                    fn $case_name() {
                        let expected: Option<PathBuf> = Some(Path::new($expected).to_owned());

                        let root = Path::new($root);
                        let repo = GitUrl::from_str($repo).expect("repo should be a URL");
                        assert_eq!(clone_destination(root, &repo).ok(), expected);
                    }
                )*
            }
        }
    }

    clone_destination_success_tests! {
        https_01: ("rel_root", "https://example.com/testrepo.git") => "rel_root/https/example.com/testrepo.git",
        https_02: ("/abs_root", "https://example.com/testrepo.git") => "/abs_root/https/example.com/testrepo.git",
    }
}

// -------------------------------------------------------------------------------------------------
// FindingSummary
// -------------------------------------------------------------------------------------------------

/// A summary of matches in a `Datastore`.
#[derive(Serialize)]
pub struct FindingSummary(pub Vec<MatchSummaryEntry>);

#[derive(Serialize)]
pub struct MatchSummaryEntry {
    pub rule_name: String,
    pub distinct_count: usize,
    pub total_count: usize,
}

impl std::fmt::Display for FindingSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for entry in self.0.iter() {
            writeln!(f, "{}: {} ({})", entry.rule_name, entry.distinct_count, entry.total_count)?;
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// FindingMetadata
// -------------------------------------------------------------------------------------------------

/// Metadata for a group of matches that have identical rule name and match content.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FindingMetadata {
    /// The content-based finding identifier for this group of matches
    pub finding_id: String,

    /// The name of the rule that detected each match
    pub rule_name: String,

    /// The textual identifier of the rule that detected each match
    pub rule_text_id: String,

    /// The structural identifier of the rule that detected each match
    pub rule_structural_id: String,

    /// The matched content of all the matches in the group
    pub groups: Groups,

    /// The number of matches in the group
    pub num_matches: usize,

    /// The unique statuses assigned to matches in the group
    pub statuses: Statuses,

    /// A comment assigned to this finding
    pub comment: Option<String>,

    /// The mean score in this group of matches
    pub mean_score: Option<f64>,
}

// -------------------------------------------------------------------------------------------------
// FindingDataEntry
// -------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct FindingDataEntry {
    pub provenance: ProvenanceSet,
    pub blob_metadata: BlobMetadata,
    pub match_id: MatchId,
    pub match_val: Match,
    pub match_comment: Option<String>,
    pub match_score: Option<f64>,
    pub match_status: Option<Status>,
}

// -------------------------------------------------------------------------------------------------
// Status
// -------------------------------------------------------------------------------------------------

/// A status assigned to a match group
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
// FIXME(overhaul): use an integer representation for serialization and db
pub enum Status {
    Accept,
    Reject,
}

// -------------------------------------------------------------------------------------------------
// Statuses
// -------------------------------------------------------------------------------------------------
/// A collection of statuses
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
// FIXME(overhaul): use a bitflag representation here?
pub struct Statuses(pub SmallVec<[Status; 16]>);

// -------------------------------------------------------------------------------------------------
// sql
// -------------------------------------------------------------------------------------------------
mod sql {
    use super::*;

    use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
    use rusqlite::Error::ToSqlConversionFailure;

    impl ToSql for Status {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            match self {
                Status::Accept => Ok("accept".into()),
                Status::Reject => Ok("reject".into()),
            }
        }
    }

    impl FromSql for Status {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            match value.as_str()? {
                "accept" => Ok(Status::Accept),
                "reject" => Ok(Status::Reject),
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    impl ToSql for Statuses {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            match serde_json::to_string(self) {
                Err(e) => Err(ToSqlConversionFailure(e.into())),
                Ok(s) => Ok(s.into()),
            }
        }
    }

    impl FromSql for Statuses {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            match value {
                ValueRef::Text(s) => {
                    serde_json::from_slice(s).map_err(|e| FromSqlError::Other(e.into()))
                }
                ValueRef::Blob(b) => {
                    serde_json::from_slice(b).map_err(|e| FromSqlError::Other(e.into()))
                }
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }
}
