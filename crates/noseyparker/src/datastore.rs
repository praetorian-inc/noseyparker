use anyhow::{bail, Context, Result};
use bstr::BString;
use indoc::indoc;
use noseyparker_rules::Rule;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span, info, trace};

use crate::blob_metadata::BlobMetadata;
use crate::git_url::GitUrl;
use crate::location::{Location, OffsetSpan, SourcePoint, SourceSpan};
use crate::match_type::Match;
use crate::provenance::Provenance;
use crate::provenance_set::ProvenanceSet;
use crate::snippet::Snippet;

const CURRENT_SCHEMA_VERSION: u64 = 70;
const CURRENT_SCHEMA: &str = include_str!("datastore/schema_70.sql");

pub mod annotation;
pub mod finding_data;
pub mod finding_metadata;
pub mod finding_summary;
pub mod status;

pub use annotation::{Annotations, FindingAnnotation, MatchAnnotation};
pub use finding_data::{FindingData, FindingDataEntry};
pub use finding_metadata::FindingMetadata;
pub use finding_summary::{FindingSummary, FindingSummaryEntry};
pub use status::{Status, Statuses};

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

        ds.migrate_0_70()
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
pub struct MatchIdInt(i64);

pub type BatchEntry = (ProvenanceSet, BlobMetadata, Vec<(Option<f64>, Match)>);

/// A datastore transaction.
/// Its lifetime parameter is for the datastore it belongs to.
pub struct Transaction<'ds> {
    inner: rusqlite::Transaction<'ds>,
}

impl<'ds> Transaction<'ds> {
    /// Commit this `Transaction`, consuming it.
    pub fn commit(self) -> Result<()> {
        self.inner.commit()?;
        Ok(())
    }

    fn mk_record_rule(&'ds self) -> Result<impl FnMut(&'ds Rule) -> rusqlite::Result<RuleIdInt>> {
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

    /// Record the given rules to the datastore.
    pub fn record_rules(&self, rules: &[Rule]) -> Result<()> {
        let mut record_rule = self.mk_record_rule()?;
        for rule in rules {
            record_rule(rule)?;
        }
        Ok(())
    }

    /// Return a closure that records a blob's metadata (only if necessary), returning its integer ID
    fn mk_record_blob_metadata(
        &'ds self,
    ) -> Result<impl FnMut(&'ds BlobMetadata) -> rusqlite::Result<BlobIdInt>> {
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
        &'ds self,
    ) -> Result<impl FnMut(BlobIdInt, &'ds Provenance) -> rusqlite::Result<()>> {
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
    fn mk_record_snippet(
        &'ds self,
    ) -> Result<impl FnMut(&'ds [u8]) -> rusqlite::Result<SnippetIdInt>> {
        let mut get = self.inner.prepare_cached(indoc! {r#"
            select id from snippet where snippet = ?
        "#})?;

        let mut set = self.inner.prepare_cached(indoc! {r#"
            insert into snippet(snippet)
            values (?)
            returning id
        "#})?;

        Ok(move |blob| {
            let id = add_if_missing_simple(&mut get, &mut set, val_from_row, (blob,))?;
            Ok(SnippetIdInt(id))
        })
    }

    /// Record a match, returning whether it was new or not
    fn mk_record_match(
        &'ds self,
    ) -> Result<impl FnMut(BlobIdInt, &'ds Match, &'ds Option<f64>) -> rusqlite::Result<bool>> {
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

        let f = move |BlobIdInt(blob_id), m: &'ds Match, score: &'ds Option<f64>| {
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
            let SnippetIdInt(before_snippet_id) = record_snippet(snippet.before.as_slice())?;
            let SnippetIdInt(matching_snippet_id) = record_snippet(snippet.matching.as_slice())?;
            let SnippetIdInt(after_snippet_id) = record_snippet(snippet.after.as_slice())?;

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

impl Datastore {
    /// Begin a new transaction.
    pub fn begin(&mut self) -> Result<Transaction> {
        let inner = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        Ok(Transaction { inner })
    }

    /// How many matches are there, total, in the datastore?
    pub fn get_num_matches(&self) -> Result<u64> {
        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select count(*) from match
        "#})?;
        let num_matches: u64 = stmt.query_row((), val_from_row)?;
        Ok(num_matches)
    }

    /// How many findings are there, total, in the datastore?
    pub fn get_num_findings(&self) -> Result<u64> {
        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select count(*) from finding
        "#})?;
        let num_findings: u64 = stmt.query_row((), val_from_row)?;
        Ok(num_findings)
    }

    /// Get a summary of all recorded findings.
    pub fn get_summary(&self) -> Result<FindingSummary> {
        let _span = debug_span!("Datastore::get_summary", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached("select * from finding_summary")?;
        let entries = stmt.query_map((), |row| {
            Ok(FindingSummaryEntry {
                rule_name: row.get(0)?,
                distinct_count: row.get(2)?,
                total_count: row.get(3)?,
                accept_count: row.get(4)?,
                reject_count: row.get(5)?,
                mixed_count: row.get(6)?,
                unlabeled_count: row.get(7)?,
            })
        })?;
        let es = collect(entries)?;
        Ok(FindingSummary(es))
    }

    /// Get annotations from this datastore.
    pub fn get_annotations(&self) -> Result<Annotations> {
        let _span =
            debug_span!("Datastore::get_annotations", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select
                md.finding_id,
                md.rule_name,
                md.rule_text_id,
                md.rule_structural_id,
                md.structural_id,
                md.blob_id,
                md.start_byte,
                md.end_byte,
                md.groups,
                md.status,
                md.comment
            from match_denorm md
            where md.status is not null or md.comment is not null
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(MatchAnnotation {
                finding_id: row.get(0)?,
                rule_name: row.get(1)?,
                rule_text_id: row.get(2)?,
                rule_structural_id: row.get(3)?,
                match_id: row.get(4)?,
                blob_id: row.get(5)?,
                start_byte: row.get(6)?,
                end_byte: row.get(7)?,
                groups: row.get(8)?,
                status: row.get(9)?,
                comment: row.get(10)?,
            })
        })?;
        let match_annotations = collect(entries)?;

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select
                md.finding_id,
                md.rule_name,
                md.rule_text_id,
                md.rule_structural_id,
                md.groups,
                md.comment
            from finding_denorm md
            where md.comment is not null
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(FindingAnnotation {
                finding_id: row.get(0)?,
                rule_name: row.get(1)?,
                rule_text_id: row.get(2)?,
                rule_structural_id: row.get(3)?,
                groups: row.get(4)?,
                comment: row.get(5)?,
            })
        })?;
        let finding_annotations = collect(entries)?;

        Ok(Annotations {
            match_annotations,
            finding_annotations,
        })
    }

    pub fn import_annotations(&mut self, annotations: &Annotations) -> Result<()> {
        #[derive(Default, Debug)]
        struct Stats {
            n_imported: usize,
            n_conflicting: usize,
            n_existing: usize,
            n_missing: usize,
        }

        impl std::fmt::Display for Stats {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{} existing; {} missing; {} conflicting; {} imported",
                    self.n_existing, self.n_missing, self.n_conflicting, self.n_imported
                )
            }
        }

        use rusqlite::{types::FromSql, CachedStatement, ToSql};

        /// This complicated helper function factors out some common "import a single annotation"
        /// logic that is common to finding comments, match comments, and match statuses.
        /// Better than repeating the code verbatim three times...?
        fn do_import<Ann, Id, Val>(
            annotation_type: &str,        // human-readable name of annotation type
            stats: &mut Stats,            // stats object to update
            getter: &mut CachedStatement, // sql getter query, takes a single `&Id` parameter
            setter: &mut CachedStatement, // sql setter query, takes an `&Id` and a `&Val` parameter
            ann: &Ann,                    // the annotation being imported
            ann_id: &Id,                  // the id from the annotation
            ann_val: &Val,                // the value from the annotation (comment, status, etc)
        ) -> Result<()>
        where
            Ann: std::fmt::Debug,
            Id: ToSql,
            Val: FromSql + ToSql + Eq + std::fmt::Debug,
        {
            use rusqlite::OptionalExtension; // for .optional()

            let existing: Option<(u64, Val)> = getter
                .query_row((ann_id,), |r| {
                    let id: u64 = r.get(0)?;
                    let val: Val = r.get(1)?;
                    Ok((id, val))
                })
                .optional()?;
            match existing {
                Some((_id, val)) if &val == ann_val => {
                    stats.n_existing += 1;
                    trace!("did not import {annotation_type}: already present: {ann:#?}");
                }
                Some((_id, val)) => {
                    stats.n_conflicting += 1;
                    debug!("did not import {annotation_type}: conflict: {val:?} {ann:#?}");
                }
                None => {
                    let n_set = setter.execute((ann_id, ann_val))?;
                    if n_set == 1 {
                        stats.n_imported += 1;
                        trace!("imported {annotation_type}: new: {ann:#?}");
                    } else {
                        assert_eq!(n_set, 0);
                        stats.n_missing += 1;
                        debug!("did not import {annotation_type}: not found: {ann:#?}");
                    }
                }
            }

            Ok(())
        }

        // Ok, now with that preamble out of the way, let's actually import the annotations

        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let mut finding_comment_stats = Stats::default();
        let mut match_comment_stats = Stats::default();
        let mut match_status_stats = Stats::default();

        // Import finding comments
        {
            let mut getter = tx.prepare_cached(indoc! {r#"
                select f.id, fc.comment
                from
                    finding f
                    inner join finding_comment fc on (fc.finding_id = f.id)
                where f.finding_id = ?
            "#})?;

            let mut setter = tx.prepare_cached(indoc! {r#"
                insert or replace into finding_comment (finding_id, comment)
                select f.id, ?2
                from finding f
                where f.finding_id = ?1
            "#})?;

            for fa in annotations.finding_annotations.iter() {
                do_import(
                    "finding comment",
                    &mut finding_comment_stats,
                    &mut getter,
                    &mut setter,
                    &fa,
                    &fa.finding_id,
                    &fa.comment,
                )?;
            }
        }

        // Import match comments
        {
            let mut getter = tx.prepare_cached(indoc! {r#"
                select m.id, mc.comment
                from
                    match m
                    inner join match_comment mc on (mc.match_id = m.id)
                where m.structural_id = ?
            "#})?;

            let mut setter = tx.prepare_cached(indoc! {r#"
                insert or replace into match_comment (match_id, comment)
                select m.id, ?2
                from match m
                where m.structural_id = ?1
            "#})?;

            for ma in annotations.match_annotations.iter() {
                let ma_comment = match &ma.comment {
                    Some(comment) => comment,
                    None => continue,
                };

                do_import(
                    "match comment",
                    &mut match_comment_stats,
                    &mut getter,
                    &mut setter,
                    &ma,
                    &ma.match_id,
                    ma_comment,
                )?;
            }
        }

        // Import match statuses
        {
            let mut getter = tx.prepare_cached(indoc! {r#"
                select m.id, ms.status
                from
                    match m
                    inner join match_status ms on (ms.match_id = m.id)
                where m.structural_id = ?
            "#})?;

            let mut setter = tx.prepare_cached(indoc! {r#"
                insert or replace into match_status (match_id, status)
                select m.id, ?2
                from match m
                where m.structural_id = ?1
            "#})?;

            for ma in annotations.match_annotations.iter() {
                let ma_status = match ma.status {
                    Some(status) => status,
                    None => continue,
                };

                do_import(
                    "match status",
                    &mut match_status_stats,
                    &mut getter,
                    &mut setter,
                    &ma,
                    &ma.match_id,
                    &ma_status,
                )?;
            }
        }

        tx.commit()?;

        info!(
            "{} findings and {} matches in datastore at {}",
            self.get_num_findings()?,
            self.get_num_matches()?,
            self.root_dir.display()
        );
        info!("Finding comment annotations: {finding_comment_stats}");
        info!("Match comment annotations: {match_comment_stats}");
        info!("Match status annotations: {match_status_stats}");

        Ok(())
    }

    /// Get metadata for all groups of identical matches recorded within this datastore.
    pub fn get_finding_metadata(
        &self,
        suppress_redundant_matches: bool,
    ) -> Result<Vec<FindingMetadata>> {
        let _span =
            debug_span!("Datastore::get_finding_metadata", "{}", self.root_dir.display()).entered();

        let query_str = format!(
            indoc! {r#"
                select
                    finding_id,
                    groups,
                    rule_structural_id,
                    rule_text_id,
                    rule_name,
                    num_matches,
                    num_redundant_matches,
                    comment,
                    match_statuses,
                    mean_score
                from finding_denorm
                where {}
                order by rule_name, rule_structural_id, mean_score desc, groups
            "#},
            if suppress_redundant_matches {
                "num_matches != num_redundant_matches"
            } else {
                "true"
            }
        );
        let mut stmt = self.conn.prepare_cached(&query_str)?;
        let entries = stmt.query_map((), |row| {
            Ok(FindingMetadata {
                finding_id: row.get(0)?,
                groups: row.get(1)?,
                rule_structural_id: row.get(2)?,
                rule_text_id: row.get(3)?,
                rule_name: row.get(4)?,
                num_matches: row.get(5)?,
                num_redundant_matches: row.get(6)?,
                comment: row.get(7)?,
                statuses: row.get(8)?,
                mean_score: row.get(9)?,
            })
        })?;
        collect(entries)
    }

    /// Get up to `max_matches` matches that belong to the finding with the given finding metadata.
    /// Each match will have up to `max_provenance_entries`.
    ///
    /// A value of `None` for either limit value means "no limit".
    pub fn get_finding_data(
        &self,
        metadata: &FindingMetadata,
        max_matches: Option<usize>,
        max_provenance_entries: Option<usize>,
        suppress_redundant_matches: bool,
    ) -> Result<FindingData> {
        let _span =
            debug_span!("Datastore::get_finding_data", "{}", self.root_dir.display()).entered();

        let match_limit: i64 = match max_matches {
            Some(max_matches) => max_matches
                .try_into()
                .expect("max_matches should be convertible"),
            None => -1,
        };

        let suppress_redundant = if suppress_redundant_matches {
            "m.id not in (select match_id from match_redundancy)"
        } else {
            "true"
        };

        let query_str = format!(
            indoc! {r#"
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
            where m.groups = ?1 and m.rule_structural_id = ?2 and {}
            order by m.blob_id, m.start_byte, m.end_byte
            limit ?3
        "#},
            suppress_redundant
        );

        let mut get_blob_metadata_and_match = self.conn.prepare_cached(&query_str)?;

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
                let id = MatchIdInt(row.get(14)?);
                let m_score = row.get(15)?;
                let m_comment = row.get(16)?;
                let m_status = row.get(17)?;
                Ok((b, id, m, m_score, m_comment, m_status))
            },
        )?;
        let mut es = Vec::new();
        for e in entries {
            let (md, id, m, match_score, match_comment, match_status) = e?;
            let ps = self.get_provenance_set(&md, max_provenance_entries)?;
            let redundant_to = self.get_redundant_to(id)?;
            es.push(FindingDataEntry {
                provenance: ps,
                blob_metadata: md,
                match_id: id,
                match_val: m,
                match_comment,
                match_score,
                match_status,
                redundant_to,
            });
        }
        Ok(es)
    }

    fn get_provenance_set(
        &self,
        metadata: &BlobMetadata,
        max_provenance_entries: Option<usize>,
    ) -> Result<ProvenanceSet> {
        let max_provenance_entries: i64 = match max_provenance_entries {
            Some(m) => m.try_into().expect("max_matches should be convertible"),
            None => -1,
        };
        let mut get = self.conn.prepare_cached(indoc! {r#"
            select provenance
            from blob_provenance_denorm
            where blob_id = ?
            order by provenance
            limit ?
        "#})?;

        let ps = get.query_map((metadata.id, max_provenance_entries), val_from_row)?;

        let results = collect(ps)?;
        match ProvenanceSet::try_from_iter(results) {
            Some(ps) => Ok(ps),
            None => bail!("At least 1 provenance entry must be provided"),
        }
    }

    /// Get the structural IDs of matches that the given one is considered redundant to
    fn get_redundant_to(&self, match_id: MatchIdInt) -> Result<Vec<String>> {
        let mut get = self.conn.prepare_cached(indoc! {r#"
            select m.structural_id
            from
                match_redundancy mr
                inner join match m on (mr.redundant_to = m.id)
            where mr.match_id = ?
            order by m.structural_id
        "#})?;

        let ids = get.query_map((match_id.0,), val_from_row)?;
        collect(ids)
    }

    fn open_impl(root_dir: &Path, cache_size: i64) -> Result<Self> {
        let db_path = root_dir.join("datastore.db");
        let conn = Self::new_connection(&db_path, cache_size)?;
        let root_dir = root_dir.to_path_buf();
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
        if user_version != CURRENT_SCHEMA_VERSION {
            bail!(
                "Unsupported schema version {user_version} (expected {}): \
                  datastores from other versions of Nosey Parker are not supported",
                CURRENT_SCHEMA_VERSION
            );
        }
        Ok(())
    }

    fn migrate_0_70(&mut self) -> Result<()> {
        let _span = debug_span!("Datastore::migrate_0_70", "{}", self.root_dir.display()).entered();
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
        if user_version > 0 && user_version < CURRENT_SCHEMA_VERSION {
            bail!(
                "This datastore has schema version {user_version}. \
                   Datastores from other Nosey Parker versions are not supported. \
                   Rescanning the inputs with a new datastore will be required."
            );
        }
        if user_version > CURRENT_SCHEMA_VERSION {
            bail!("Unknown schema version {user_version}");
        }

        if user_version == 0 {
            let new_user_version = CURRENT_SCHEMA_VERSION;
            debug!("Migrating database schema from version {user_version} to {new_user_version}");
            tx.execute_batch(CURRENT_SCHEMA)?;
            set_user_version(new_user_version)?;
        }

        assert_eq!(get_user_version()?, CURRENT_SCHEMA_VERSION);
        tx.commit()?;

        Ok(())
    }

    /// Analyze the recorded matches to determine which matches are redundant.
    /// This populates the `match_redundancy` table.
    /// This information is needed for suppressing redundant matches at reporting time.
    pub fn check_match_redundancies(&mut self) -> Result<()> {
        self.conn.execute(indoc! {r#"
            insert or ignore into match_redundancy (match_id, redundant_to)
            with
                match_overlap_metadata as (
                    select
                        match.id,
                        finding.rule_id in generic_rule_id generic,
                        finding.rule_id in fuzzy_rule_id fuzzy,
                        rpl.length pattern_len,
                        json_array_length(finding.groups) num_groups,
                        length(finding.groups) groups_len,
                        match.end_byte - match.start_byte match_len
                    from
                        match
                        inner join finding on (match.finding_id = finding.id)
                        inner join rule_pattern_length rpl on (finding.rule_id = rpl.rule_id)
                ),

                ordered_overlapping_match_ids as (
                    select m1.id m1_id, m2.id m2_id
                    from
                        match m1
                        inner join match m2 on (
                                m1.blob_id = m2.blob_id
                            and m1.id != m2.id
                            and (m1.start_byte <= m2.start_byte and m2.start_byte < m1.end_byte)
                            -- require at least 20% of both matches to be overlapping to be considered as an overlap
                            and (cast(m1.end_byte - m2.start_byte as real) / cast(m1.end_byte - m1.start_byte as real) >= 0.20)
                            and (cast(m1.end_byte - m2.start_byte as real) / cast(m2.end_byte - m2.start_byte as real) >= 0.20)
                        )
                ),

                overlapping_match_ids (m1_id, m2_id) as (
                    select distinct * from (
                        select m1_id, m2_id from ordered_overlapping_match_ids
                            union all
                        select m2_id, m1_id from ordered_overlapping_match_ids
                    )
                )

            select
                o.m1_id,
                o.m2_id
            from
                overlapping_match_ids o
                inner join match_overlap_metadata md1 on (o.m1_id = md1.id)
                inner join match_overlap_metadata md2 on (o.m2_id = md2.id)
            where
                -- a generic match can only replace another generic one
                (not md2.generic or md1.generic)
                    and
                -- a match can only replace one if it has at least as many groups
                (md2.num_groups >= md1.num_groups)
                    and
                (
                    -- a match can replace another with the same number of groups if any of the following apply:
                    -- * its group content is longer
                    -- * it is not fuzzy
                    -- * its group length is the same but it matches a shorter amount of overall input
                    -- * it has a longer pattern
                    not (md2.num_groups = md1.num_groups) or (
                        md2.groups_len > md1.groups_len
                            or
                        not md2.fuzzy
                            or
                        (md2.groups_len = md1.groups_len and md2.match_len < md1.match_len)
                            or
                        md2.pattern_len > md1.pattern_len
                    )
                )
        "#}, [])?;

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// Implementation Utilities
// -------------------------------------------------------------------------------------------------

fn collect<T, F>(rows: rusqlite::MappedRows<'_, F>) -> Result<Vec<T>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

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
