use anyhow::{bail, Context, Result};
use bstr::BString;
use indoc::indoc;
use input_enumerator::git_commit_metadata::CommitMetadata;
use noseyparker_rules::Rule;
use rusqlite::{types::FromSqlError, Connection};
use serde::Serialize;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span};

use crate::blob_id::BlobId;
use crate::blob_metadata::BlobMetadata;
use crate::git_url::GitUrl;
use crate::location::{Location, OffsetSpan, SourcePoint, SourceSpan};
use crate::match_type::{Groups, Match};
use crate::provenance::{CommitKind, Provenance, ProvenanceKind};
use crate::provenance_set::ProvenanceSet;
use crate::snippet::Snippet;

const SCHEMA: &str = include_str!("datastore/schema.sql");

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

#[derive(Copy, Clone, Debug)]
struct BlobIdInt(i64);

#[derive(Copy, Clone, Debug)]
struct RuleIdInt(i64);

#[derive(Copy, Clone, Debug)]
struct SnippetIdInt(i64);

#[derive(Copy, Clone, Debug)]
struct MatchIdInt(i64);

type BatchEntry = (ProvenanceSet, BlobMetadata, Vec<Match>);

pub struct Transaction<'a> {
    inner: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    pub fn commit(self) -> Result<()> {
        self.inner.commit()?;
        Ok(())
    }

    fn mk_record_rule(&'a self) -> Result<impl FnMut(&'a Rule) -> rusqlite::Result<RuleIdInt>> {
        let mut get_id = self.inner.prepare_cached(indoc! {r#"
            select id from rule
            where structural_id = ? and name = ? and text_id = ?
        "#})?;

        let mut set_id = self.inner.prepare_cached(indoc! {r#"
            insert into rule(structural_id, name, text_id)
            values (?, ?, ?)
            returning id
        "#})?;

        let mut record_json_syntax = self.inner.prepare_cached(indoc! {r#"
            insert into rule_syntax(rule_id, syntax)
            values (?, ?)
            on conflict do update set syntax = excluded.syntax
            where syntax != excluded.syntax
        "#})?;

        let f = move |r: &Rule| -> rusqlite::Result<RuleIdInt> {
            let rule_id =
                add_if_missing_simple(&mut get_id, &mut set_id, val_from_row, (r.structural_id(), r.name(), r.id()))?;
            let json_syntax = r.syntax().to_json();
            record_json_syntax.execute((rule_id, json_syntax))?;
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
            let blob_id = add_if_missing_simple(&mut get_id, &mut set_id, val_from_row, (&b.id.hex(), b.num_bytes))?;

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
            let provenance_json = serde_json::to_string(provenance).expect("should be able to serialize provenance as JSON");
            add_provenance.execute((blob_id, provenance_json))?;
            Ok(())
        };

        Ok(f)
    }

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

    fn mk_record_match_snippet(
        &'a self,
    ) -> Result<impl FnMut(MatchIdInt, &'a Snippet) -> rusqlite::Result<()>> {
        let mut record_snippet = self.mk_record_snippet()?;

        let mut record_match_snippet = self.inner.prepare_cached(indoc! {r#"
            insert into match_snippet(
                match_id,
                before_snippet_id,
                matching_snippet_id,
                after_snippet_id
            )
            values (?, ?, ?, ?)
            on conflict do update set
                before_snippet_id = excluded.before_snippet_id,
                matching_snippet_id = excluded.matching_snippet_id,
                after_snippet_id = excluded.after_snippet_id
            where
                before_snippet_id != excluded.before_snippet_id
                or matching_snippet_id != excluded.matching_snippet_id
                or after_snippet_id != excluded.after_snippet_id
        "#})?;

        let f = move |MatchIdInt(match_id), snippet: &'a Snippet| -> rusqlite::Result<()> {
            let before_id = record_snippet(snippet.before.as_slice())?;
            let matching_id = record_snippet(snippet.matching.as_slice())?;
            let after_id = record_snippet(snippet.after.as_slice())?;
            record_match_snippet.execute((match_id, before_id, matching_id, after_id))?;
            Ok(())
        };

        Ok(f)
    }

    /// Record matches
    fn mk_record_match(
        &'a self,
    ) -> Result<impl FnMut(BlobIdInt, &'a Match) -> rusqlite::Result<bool>> {
        let mut record_match_snippet = self.mk_record_match_snippet()?;

        let mut get_match_id = self.inner.prepare_cached(indoc! {r#"
            select m.id, false
            from match m
            inner join rule r on (m.rule_id = r.id)
            where
                m.blob_id = ?
                and m.start_byte = ?
                and m.end_byte = ?
                and r.structural_id = ?

        "#})?;

        let mut set_match_id = self.inner.prepare_cached(indoc! {r#"
            insert into match (blob_id, start_byte, end_byte, rule_id)
            select ?, ?, ?, r.id
            from rule r
            where r.structural_id = ?
            returning id, true
        "#})?;

        let mut set_structural_id = self.inner.prepare_cached(indoc! {r#"
            insert into match_structural_id (match_id, structural_id)
            values (?, ?)
            on conflict do update set structural_id = excluded.structural_id
            where structural_id != excluded.structural_id
        "#})?;

        let mut set_finding_id = self.inner.prepare_cached(indoc! {r#"
            insert into match_finding_id (match_id, finding_id)
            values (?, ?)
            on conflict do update set finding_id = excluded.finding_id
            where finding_id != excluded.finding_id
        "#})?;

        let mut set_blob_source_span = self.inner.prepare_cached(indoc! {r#"
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

        let mut set_groups = self.inner.prepare_cached(indoc! {r#"
            insert into match_groups (match_id, groups)
            values (?, ?)
            on conflict do update set groups = excluded.groups
            where groups != excluded.groups
        "#})?;

        let f = move |BlobIdInt(blob_id), m: &'a Match| {
            let start_byte = m.location.offset_span.start;
            let end_byte = m.location.offset_span.end;

            let (match_id, new) = add_if_missing_simple(
                &mut get_match_id,
                &mut set_match_id,
                from_row,
                (blob_id, start_byte, end_byte, &m.rule_structural_id),
            )?;

            record_match_snippet(MatchIdInt(match_id), &m.snippet)?;

            let structural_id = &m.structural_id();
            set_structural_id.execute((match_id, structural_id))?;

            let finding_id = &m.finding_id();
            set_finding_id.execute((match_id, finding_id))?;

            let start_line = m.location.source_span.start.line;
            let start_column = m.location.source_span.start.column;
            let end_line = m.location.source_span.end.line;
            let end_column = m.location.source_span.end.column;
            set_blob_source_span.execute((blob_id, start_byte, end_byte, start_line, start_column, end_line, end_column))?;

            let groups_json = serde_json::to_string(&m.groups).expect("should be able to serialize groups as JSON");
            set_groups.execute((match_id, groups_json))?;

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
                record_provenance(blob_id, p)
                    .context("Failed to record blob provenance")?;
            }

            // record matches
            for m in ms {
                if record_match(blob_id, m).context("Failed to record match")? {
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
    pub fn summarize(&self) -> Result<MatchSummary> {
        let _span = debug_span!("Datastore::summarize", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select rule_name, total_findings, total_matches
            from finding_summary
            order by total_findings desc
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
        Ok(MatchSummary(es))
    }

    /// Get metadata for all groups of identical matches recorded within this `Datastore`.
    pub fn get_finding_metadata(&self) -> Result<Vec<FindingMetadata>> {
        let _span =
            debug_span!("Datastore::get_finding_metadata", "{}", self.root_dir.display())
                .entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select groups, rule_name, num_matches, null, null -- TODO: add comment and status
            from finding_denorm
            order by rule_name
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(FindingMetadata {
                groups: serde_json::from_str(row.get_ref(0)?.as_str()?).map_err(|e| FromSqlError::Other(e.into()))?,
                rule_name: row.get(1)?,
                num_matches: row.get(2)?,
                comment: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        let mut es = Vec::new();
        for e in entries {
            es.push(e?);
        }
        Ok(es)
    }

    /// Get up to `limit` matches that belong to the group with the given group metadata.
    pub fn get_match_group_data(
        &self,
        metadata: &FindingMetadata,
        limit: Option<usize>,
    ) -> Result<Vec<(ProvenanceSet, BlobMetadata, MatchId, Match)>> {
        let _span =
            debug_span!("Datastore::get_match_group_data", "{}", self.root_dir.display()).entered();

        let match_limit: i64 = match limit {
            Some(limit) => limit.try_into().expect("limit should be convertible"),
            None => -1,
        };

        let mut get_blob_metadata_and_match = self.conn.prepare_cached(indoc! {r#"
            select
                b.blob_id,
                m.start_byte,
                m.end_byte,
                m.start_line,
                m.start_column,
                m.end_line,
                m.end_column,
                m.before_snippet,
                m.matching_input,
                m.after_snippet,
                m.group_index,

                b.size,
                b.mime_essence,
                b.charset,

                m.id

            from match m
            inner join blob b on (m.blob_id = b.id)
            where m.group_input = ?1 and m.rule_name = ?2
            order by m.blob_id, m.start_byte, m.end_byte
            limit ?3
        "#})?;

        let entries = get_blob_metadata_and_match.query_map(
            // (metadata.groups.as_slice(), &metadata.rule_name, match_limit),
            ("", &metadata.rule_name, match_limit),
            |row| {
                let v0: String = row.get(0)?;
                let blob_id = BlobId::from_hex(&v0).map_err(|e| FromSqlError::Other(e.into()))?;
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
                    groups: Default::default(),
                    // capture_group_index: row.get(10)?,
                    // match_content: metadata.match_content.clone(),
                    // rule_name: metadata.rule_name.clone(),
                    rule_structural_id: metadata.rule_name.clone(),
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
                Ok((b, id, m))
            },
        )?;
        let mut es = Vec::new();
        for e in entries {
            let (md, id, m) = e?;
            let ps = self.get_provenance_set(&md)?;
            es.push((ps, md, id, m));
        }
        Ok(es)
    }

    fn get_provenance_set(&self, metadata: &BlobMetadata) -> Result<ProvenanceSet> {
        let mut get_provenance = self.conn.prepare_cached(indoc! {r#"
            select distinct p.payload_kind, p.payload_id, bp.kind provenance_kind
            from
                blob_provenance bp
                inner join blob b on (bp.blob_id = b.id)
                inner join provenance p on (bp.provenance_id = p.id)
            where b.blob_id = ?
        "#})?;

        let mut get_provenance_payload_file = self.conn.prepare_cached(indoc! {r#"
            select path from provenance_payload_file
            where id = ?
        "#})?;

        let mut get_provenance_payload_git_repo = self.conn.prepare_cached(indoc! {r#"
            select repo_path from provenance_payload_git_repo
            where id = ?
        "#})?;

        let mut get_provenance_payload_git_commit = self.conn.prepare_cached(indoc! {r#"
            select repo_path, commit_id, blob_path
            from provenance_payload_git_commit
            where id = ?
        "#})?;

        let mut get_commit_metadata = self.conn.prepare_cached(indoc! {r#"
            select
                commit_id,
                committer_name,
                committer_email,
                commit_date,
                author_name,
                author_email,
                author_date,
                message
            from git_commit
            where id = ?
        "#})?;

        let ps = get_provenance.query_map((metadata.id.hex(),), |row| {
            let payload_kind = row.get(0)?;
            let payload_id: i64 = row.get(1)?;
            let provenance_kind: Option<CommitKind> = row.get(2)?;
            match payload_kind {
                ProvenanceKind::File => {
                    let path: Vec<u8> =
                        get_provenance_payload_file.query_row((payload_id,), |r| r.get(0))?;
                    let path = PathBuf::from(OsString::from_vec(path));
                    Ok(Provenance::from_file(path))
                }

                ProvenanceKind::GitRepo => {
                    let path: Vec<u8> =
                        get_provenance_payload_git_repo.query_row((payload_id,), |r| r.get(0))?;
                    let path = PathBuf::from(OsString::from_vec(path));
                    Ok(Provenance::from_git_repo(path))
                }

                ProvenanceKind::GitCommit => {
                    let (repo_path, commit_id, blob_path) = get_provenance_payload_git_commit
                        .query_row((payload_id,), |row| {
                            let repo_path: Vec<u8> = row.get(0)?;
                            let repo_path = PathBuf::from(OsString::from_vec(repo_path));

                            let commit_id: i64 = row.get(1)?;

                            let blob_path: Vec<u8> = row.get(2)?;
                            let blob_path = BString::from(blob_path);

                            Ok((repo_path, commit_id, blob_path))
                        })?;

                    let commit_metadata: CommitMetadata =
                        get_commit_metadata.query_row((commit_id,), |row| {
                            let get_bstring = |idx: usize| -> rusqlite::Result<BString> {
                                Ok(row.get_ref(idx)?.as_bytes()?.into())
                            };

                            let get_time = |idx: usize| -> rusqlite::Result<gix::date::Time> {
                                let epoch_seconds = row.get_ref(idx)?.as_i64()?;
                                Ok(gix::date::Time::new(epoch_seconds, 0))
                            };

                            let commit_id = gix::ObjectId::from_hex(row.get_ref(0)?.as_bytes()?)
                                .map_err(|e| FromSqlError::Other(e.into()))?;

                            let committer_name = get_bstring(1)?;
                            let committer_email = get_bstring(2)?;
                            let committer_timestamp = get_time(3)?;
                            let author_name = get_bstring(4)?;
                            let author_email = get_bstring(5)?;
                            let author_timestamp = get_time(6)?;
                            let message = get_bstring(7)?;

                            Ok(CommitMetadata {
                                commit_id,
                                committer_name,
                                committer_email,
                                committer_timestamp,
                                author_name,
                                author_email,
                                author_timestamp,
                                message,
                            })
                        })?;

                    let provenance_kind = provenance_kind.ok_or(FromSqlError::InvalidType)?;
                    Ok(Provenance::from_git_repo_and_commit_metadata(
                        repo_path,
                        provenance_kind,
                        commit_metadata,
                        blob_path,
                    ))
                }
            }
        })?;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MatchId(pub i64);

/// A function that exists to make SQL row conversion to a value via TryFrom<&rusqlite::Row> more
/// ergonomic.
fn from_row<T>(row: &rusqlite::Row<'_>) -> rusqlite::Result<T>
where
    T: for<'a> TryFrom<&'a rusqlite::Row<'a>, Error = rusqlite::Error>,
{
    T::try_from(row)
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
fn add_if_missing_simple<'a, P, F, T>(
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
fn add_if_missing<'a, P1, P2, F, T>(
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
        // let (user_version, ): (u64, ) = self.conn.pragma_query_value(None, "user_version", |r| <(u64, )>::try_from(r))?;
        let user_version: u64 = self
            .conn
            .pragma_query_value(None, "user_version", val_from_row)?;
        if user_version != Self::CURRENT_SCHEMA_VERSION {
            bail!("Unsupported schema version {user_version}");
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
            tx.execute_batch(SCHEMA)?;
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
// MatchSummary
// -------------------------------------------------------------------------------------------------

/// A summary of matches in a `Datastore`.
#[derive(Serialize)]
pub struct MatchSummary(pub Vec<MatchSummaryEntry>);

#[derive(Serialize)]
pub struct MatchSummaryEntry {
    pub rule_name: String,
    pub distinct_count: usize,
    pub total_count: usize,
}

impl std::fmt::Display for MatchSummary {
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
#[derive(Debug, Serialize)]
pub struct FindingMetadata {
    /// The name of the rule of all the matches in the group
    pub rule_name: String,

    /// The matched content of all the matches in the group
    pub groups: Groups,

    /// The number of matches in the group
    pub num_matches: usize,

    /// An optional status assigned to this match group
    pub status: Option<Status>,

    /// A comment assigned to this match group
    pub comment: Option<String>,
}

// -------------------------------------------------------------------------------------------------
// Status
// -------------------------------------------------------------------------------------------------

/// A status assigned to a match group
#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Accept,
    Reject,
}

impl rusqlite::types::ToSql for Status {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Status::Accept => Ok("accept".into()),
            Status::Reject => Ok("reject".into()),
        }
    }
}

impl rusqlite::types::FromSql for Status {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "accept" => Ok(Status::Accept),
            "reject" => Ok(Status::Reject),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}
