use anyhow::{bail, Context, Result};
use bstr::BString;
use bstring_serde::BStringSerde;
use indoc::indoc;
use input_enumerator::git_commit_metadata::CommitMetadata;
use rusqlite::{Connection, types::FromSqlError};
use serde::Serialize;
use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span};

use crate::blob_id::BlobId;
use crate::blob_metadata::BlobMetadata;
use crate::git_url::GitUrl;
use crate::location::{Location, OffsetSpan, SourcePoint, SourceSpan};
use crate::match_type::Match;
use crate::provenance::{CommitKind, CommitProvenance, Provenance, ProvenanceKind};
use crate::provenance_set::ProvenanceSet;
use crate::snippet::Snippet;

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
        Self::create(root_dir, cache_size)
            .or_else(|_e| Self::open(root_dir, cache_size))
            .with_context(|| format!("Failed to open datastore at {}", root_dir.display()))
    }

    /// Open the existing datastore at `root_dir`.
    pub fn open(root_dir: &Path, cache_size: i64) -> Result<Self> {
        let db_path = root_dir.join("datastore.db");
        let conn = Self::new_connection(&db_path, cache_size)
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?;
        let root_dir = root_dir.canonicalize().with_context(|| {
            format!("Failed to canonicalize datastore path at {}", root_dir.display())
        })?;
        let mut ds = Self { root_dir, conn };
        ds.migrate()
            .with_context(|| format!("Failed to migrate database at {}", db_path.display()))?;

        let scratch_dir = ds.scratch_dir();
        std::fs::create_dir_all(&scratch_dir).with_context(|| {
            format!(
                "Failed to create scratch directory {} for datastore at {}",
                scratch_dir.display(),
                ds.root_dir().display()
            )
        })?;

        let clones_dir = ds.clones_dir();
        std::fs::create_dir_all(&clones_dir).with_context(|| {
            format!(
                "Failed to create clones directory {} for datastore at {}",
                clones_dir.display(),
                ds.root_dir().display()
            )
        })?;

        let blobs_dir = ds.blobs_dir();
        std::fs::create_dir_all(&blobs_dir).with_context(|| {
            format!(
                "Failed to create blobs directory {} for datastore at {}",
                blobs_dir.display(),
                ds.root_dir().display()
            )
        })?;

        Ok(ds)
    }

    /// Create a new datastore at `root_dir` and open it.
    pub fn create(root_dir: &Path, cache_size: i64) -> Result<Self> {
        // Create datastore directory
        std::fs::create_dir(root_dir).with_context(|| {
            format!("Failed to create datastore root directory at {}", root_dir.display())
        })?;

        // Generate .gitignore file
        std::fs::write(root_dir.join(".gitignore"), "*\n").with_context(|| {
            format!("Failed to write .gitignore to datastore at {}", root_dir.display())
        })?;

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

type BatchEntry = (ProvenanceSet, BlobMetadata, Vec<Match>);

pub struct Transaction<'a> {
    inner: rusqlite::Transaction<'a>,
}

impl <'a> Transaction<'a> {
    pub fn commit(self) -> Result<()> {
        self.inner.commit()?;
        Ok(())
    }

    /// Record the given data into the datastore.
    /// Returns the number of matches that were newly added.
    pub fn record(&self, batch: &[BatchEntry]) -> Result<u64> {
        let mut add_blob_get_id = self.inner.prepare_cached(indoc! {r#"
            insert into blob(blob_id, size, mime_essence, charset)
            values (?, ?, ?, ?)
            on conflict do update set
                size = excluded.size,
                mime_essence = coalesce(excluded.mime_essence, mime_essence),
                charset = coalesce(excluded.charset, charset)
            returning id
        "#})?;

        let mut add_git_commit = {
            let mut get = self.inner.prepare_cached(indoc! {r#"
                select id from git_commit where commit_id = ?
            "#})?;

            let mut set = self.inner.prepare_cached(indoc! {r#"
                insert into git_commit(
                    commit_id,
                    commit_date,
                    committer_name,
                    committer_email,
                    author_date,
                    author_name,
                    author_email,
                    message
                ) values (?, ?, ?, ?, ?, ?, ?, ?)
                returning id
            "#})?;

            move |c: &CommitProvenance| -> rusqlite::Result<i64> {
                let commit_id = c.commit_metadata.commit_id.to_string();
                let params = (
                    &commit_id,
                    c.commit_metadata.committer_timestamp.seconds,
                    c.commit_metadata.committer_name.as_slice(),
                    c.commit_metadata.committer_email.as_slice(),
                    c.commit_metadata.author_timestamp.seconds,
                    c.commit_metadata.author_name.as_slice(),
                    c.commit_metadata.author_email.as_slice(),
                    c.commit_metadata.message.as_slice(),
                );
                get.query((&commit_id,))?
                    .next()?
                    .map_or_else(|| set.query_row(params, |row| row.get(0)), |row| row.get(0))
            }
        };

        let mut add_match = {
            let mut get = self.inner.prepare_cached(indoc! {r#"
                select
                    start_line,
                    start_column,
                    end_line,
                    end_column,
                    before_snippet,
                    matching_input,
                    after_snippet,
                    group_input
                from match
                where
                    blob_id = ? and
                    start_byte = ? and
                    end_byte = ? and
                    group_index = ? and
                    rule_name = ?
            "#})?;

            let mut set = self.inner.prepare_cached(indoc! {r#"
                insert into match(
                    blob_id,
                    start_byte,
                    end_byte,
                    group_index,
                    rule_name,

                    start_line,
                    start_column,
                    end_line,
                    end_column,
                    before_snippet,
                    matching_input,
                    after_snippet,
                    group_input
                ) values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                on conflict do update set
                    start_line = excluded.start_line,
                    start_column = excluded.start_column,
                    end_line = excluded.end_line,
                    end_column = excluded.end_column,
                    before_snippet = excluded.before_snippet,
                    matching_input = excluded.matching_input,
                    after_snippet = excluded.after_snippet,
                    group_input = excluded.group_input
            "#})?;

            move |blob_id: i64, m: &Match| -> rusqlite::Result<bool> {
                let params = (
                    blob_id,
                    m.location.offset_span.start,
                    m.location.offset_span.end,
                    m.capture_group_index,
                    &m.rule_name,
                );

                let full_params = (
                    blob_id,
                    m.location.offset_span.start,
                    m.location.offset_span.end,
                    m.capture_group_index,
                    &m.rule_name,

                    m.location.source_span.start.line,
                    m.location.source_span.start.column,
                    m.location.source_span.end.line,
                    m.location.source_span.end.column,
                    m.snippet.before.as_slice(),
                    m.snippet.matching.as_slice(),
                    m.snippet.after.as_slice(),
                    m.match_content.as_slice(),
                );

                let mut new = false;
                // to reduce database writes, only insert the match data if it's meaningfully
                // different than what's there already
                let different = match get.query(params)?.next()? {
                    None => {
                        new = true;
                        true
                    },
                    Some(row) => {
                        row.get_ref(0)?.as_i64()? != full_params.5 as i64 ||
                        row.get_ref(1)?.as_i64()? != full_params.6 as i64 ||
                        row.get_ref(2)?.as_i64()? != full_params.7 as i64 ||
                        row.get_ref(3)?.as_i64()? != full_params.8 as i64 ||
                        row.get_ref(4)?.as_bytes()? != full_params.9 ||
                        row.get_ref(5)?.as_bytes()? != full_params.10 ||
                        row.get_ref(6)?.as_bytes()? != full_params.11 ||
                        row.get_ref(7)?.as_bytes()? != full_params.12
                    }
                };

                if different {
                    set.execute(full_params)?;
                }
                Ok(new)
            }
        };

        let mut add_provenance_payload_file = {
            let mut set = self.inner.prepare_cached(indoc! {r#"
                insert into provenance_payload_file(path)
                values (?)
                returning id
            "#})?;

            let mut get = self.inner.prepare_cached(indoc! {r#"
                select id from provenance_payload_file
                where path = ?
            "#})?;

            move |path: &[u8]| -> rusqlite::Result<i64> {
                let params = (path,);
                get.query(params)?
                    .next()?
                    .map_or_else(|| set.query_row(params, |r| r.get(0)), |row| row.get(0))
            }
        };

        let mut add_provenance_payload_git_repo = {
            let mut get = self.inner.prepare_cached(indoc! {r#"
                select id from provenance_payload_git_repo
                where repo_path = ?
            "#})?;

            let mut set = self.inner.prepare_cached(indoc! {r#"
                insert into provenance_payload_git_repo(repo_path)
                values (?)
                returning id
            "#})?;

            move |repo_path: &[u8]| -> rusqlite::Result<i64> {
                let params = (repo_path,);
                get.query(params)?
                    .next()?
                    .map_or_else(|| set.query_row(params, |r| r.get(0)), |row| row.get(0))
            }
        };

        let mut add_provenance_payload_git_commit = {
            let mut get = self.inner.prepare_cached(indoc! {r#"
                select id from provenance_payload_git_commit
                where repo_path = ? and commit_id = ? and blob_path = ?
            "#})?;

            let mut set = self.inner.prepare_cached(indoc! {r#"
                insert into provenance_payload_git_commit(repo_path, commit_id, blob_path)
                values (?, ?, ?)
                returning id
            "#})?;

            move |repo_path: &[u8], commit_id: i64, blob_path: &[u8]| -> rusqlite::Result<i64> {
                let params = (repo_path, commit_id, blob_path);
                get.query(params)?
                    .next()?
                    .map_or_else(|| set.query_row(params, |r| r.get(0)), |row| row.get(0))
            }
        };

        let mut add_provenance = {
            let mut set = self.inner.prepare_cached(indoc! {r#"
                insert into provenance(payload_kind, payload_id)
                values (?, ?)
                returning id
            "#})?;

            let mut get = self.inner.prepare_cached(indoc! {r#"
                select id from provenance
                where payload_kind = ? and payload_id = ?
            "#})?;

            move |kind: ProvenanceKind, payload_id: i64| -> rusqlite::Result<i64> {
                let params = (kind, payload_id);
                get.query(params)?
                    .next()?
                    .map_or_else(|| set.query_row(params, |r| r.get(0)), |row| row.get(0))
            }
        };

        let mut add_provenance_id = self.inner.prepare_cached(indoc! {r#"
            insert into blob_provenance(blob_id, provenance_id, kind)
            values (?, ?, ?)
            on conflict do nothing
        "#})?;

        let mut num_matches_added = 0;

        for (ps, md, ms) in batch {
            // record blob metadata
            let blob_id: i64 = add_blob_get_id
                .query_row((md.id.hex(), md.num_bytes(), md.mime_essence(), md.charset()), |r| {
                    r.get(0)
                })
                .context("Failed to add blob metadata")?;

            // record provenance
            for p in ps.iter() {
                let (provenance_id, kind) = match p {
                    Provenance::File(e) => {
                        let path = e.path.as_os_str().as_bytes();
                        let payload_id = add_provenance_payload_file(path)?;
                        let provenance_id = add_provenance(ProvenanceKind::File, payload_id)?;
                        (provenance_id, None)
                    }
                    Provenance::GitRepo(e) => {
                        let repo_path = e.repo_path.as_os_str().as_bytes();
                        match &e.commit_provenance {
                            // no commit information; record just the repo
                            None => {
                                let payload_id = add_provenance_payload_git_repo(repo_path)?;
                                let provenance_id =
                                    add_provenance(ProvenanceKind::GitRepo, payload_id)?;
                                (provenance_id, None)
                            }
                            // detailed commit information
                            Some(c) => {
                                let commit_id = add_git_commit(c)?;
                                let blob_path = c.blob_path.as_slice();
                                let payload_id = add_provenance_payload_git_commit(
                                    repo_path, commit_id, blob_path,
                                )?;
                                let provenance_id =
                                    add_provenance(ProvenanceKind::GitCommit, payload_id)?;
                                (provenance_id, Some(c.commit_kind))
                            }
                        }
                    }
                };

                add_provenance_id.execute((blob_id, provenance_id, kind))?;
            }

            // record matches
            for m in ms {
                if add_match(blob_id, m)? {
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
        let num_matches: u64 = stmt.query_row((), |row| row.get(0))?;
        Ok(num_matches)
    }

    /// Summarize all recorded findings.
    pub fn summarize(&self) -> Result<MatchSummary> {
        let _span = debug_span!("Datastore::summarize", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select rule_name, count(*) grouped_count, sum(num_matches) total_count
            from (
                select group_input, rule_name, count(*) num_matches
                from match
                group by 1, 2
            )
            group by 1
            order by grouped_count desc
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
    pub fn get_match_group_metadata(&self) -> Result<Vec<MatchGroupMetadata>> {
        let _span =
            debug_span!("Datastore::get_match_group_metadata", "{}", self.root_dir.display())
                .entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select m.group_input, m.rule_name, count(*), fc.comment, fs.status
            from match m
            left outer join finding_comment fc on (m.group_input = fc.group_input and m.rule_name = fc.rule_name)
            left outer join finding_status fs on (m.group_input = fs.group_input and m.rule_name = fs.rule_name)
            group by 1, 2
            order by 2
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(MatchGroupMetadata {
                match_content: BString::new(row.get(0)?),
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
        metadata: &MatchGroupMetadata,
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
            (metadata.match_content.as_slice(), &metadata.rule_name, match_limit),
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
                    capture_group_index: row.get(10)?,
                    match_content: metadata.match_content.clone(),
                    rule_name: metadata.rule_name.clone(),
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

// Private implementation
impl Datastore {
    fn new_connection(path: &Path, cache_size: i64) -> Result<Connection> {
        let conn = Connection::open(path)?;

        conn.pragma_update(None, "journal_mode", "wal")?; // https://www.sqlite.org/wal.html
        conn.pragma_update(None, "foreign_keys", "on")?; // https://sqlite.org/foreignkeys.html
        conn.pragma_update(None, "synchronous", "normal")?; // https://sqlite.org/pragma.html#pragma_synchronous
        conn.pragma_update(None, "cache_size", cache_size)?; // sqlite.org/pragma.html#pragma_cache_size

        Ok(conn)
    }

    fn migrate(&mut self) -> Result<()> {
        let _span = debug_span!("Datastore::migrate", "{}", self.root_dir.display()).entered();
        let tx = self.conn.transaction()?;

        let get_user_version = || -> Result<u64> {
            let user_version = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
            Ok(user_version)
        };

        let set_user_version = |user_version: u64| -> Result<()> {
            tx.pragma_update(None, "user_version", user_version)?;
            Ok(())
        };

        let user_version: u64 = get_user_version()?;
        if user_version == 1 || user_version == 2 {
            bail!(
                "Datastores from earlier Nosey Parker versions cannot be migrated to \
                  the new format; rescanning the inputs with a new datastore will be required."
            );
        }
        if user_version > 5 {
            bail!("Unknown schema version {user_version}");
        }

        // -----------------------------------------------------------------------------------------
        // migration 3
        // -----------------------------------------------------------------------------------------
        if user_version == 0 {
            let new_user_version = 3;
            debug!("Migrating database schema from version {user_version} to {new_user_version}");
            tx.execute_batch(indoc! {r#"
                create table blob
                -- This table records various bits of metadata about blobs.
                (
                    id integer primary key,

                    -- The blob hash, computed a la Git
                    blob_id text unique not null,

                    -- Size of the blob in bytes
                    size integer not null,

                    -- Guessed mime type of the blob
                    mime_essence text,

                    -- Guess charset encoding of the blob
                    charset text,

                    constraint valid_blob_id check(
                        length(blob_id) == 40 and not glob('*[^abcdefABCDEF1234567890]*', blob_id)
                    ),
                    constraint valid_size check(0 <= size)
                ) strict;

                create table git_commit
                -- This table records various bits of metadata about Git commits.
                (
                    id integer primary key,

                    -- The commit hash
                    commit_id text unique not null,

                    -- The commit timestamp, in seconds since the epoch
                    commit_date integer not null,

                    -- The committer name
                    committer_name blob not null,

                    -- The committer email
                    committer_email blob not null,

                    -- The commit author timestamp, in seconds since the epoch
                    author_date integer not null,

                    -- The commit author
                    author_name blob not null,

                    -- The commit author
                    author_email blob not null,

                    -- The commit message
                    message blob not null,

                    constraint valid_commit_id check(
                        length(commit_id) == 40 and not glob('*[^abcdefABCDEF1234567890]*', commit_id)
                    )
                ) strict;

                create table provenance_payload_file
                -- This table records provenance information about plain files.
                (
                    id integer primary key,

                    -- The filesystem path of the file
                    path blob unique not null
                ) strict;

                create table provenance_payload_git_commit
                -- This table records provenance information about Git commits.
                (
                    id integer primary key,

                    -- The filesystem path of the Git repo
                    repo_path blob not null,

                    commit_id integer not null references git_commit(id),

                    -- The path of the blob within the commit
                    blob_path blob not null,

                    unique(repo_path, commit_id, blob_path)
                ) strict;

                create table provenance_payload_git_repo
                -- This table records provenance information about Git repositories.
                (
                    id integer primary key,

                    -- The filesystem path of the Git repo
                    repo_path blob not null,

                    unique(repo_path)
                ) strict;

                create table provenance
                -- This table encodes a union of the `provenance_payload_*` tables.
                (
                    id integer primary key,

                    payload_kind text not null,
                    -- The ID of the provenance payload;
                    -- references one of the `provenance_payload_*` tables depending on `kind`
                    payload_id integer not null,

                    unique(payload_kind, payload_id),

                    constraint valid_payload_kind check(payload_kind in ('file', 'git_repo', 'git_commit'))
                ) strict;

                create table blob_provenance
                -- This table records the various ways in which a blob was encountered.
                (
                    id integer primary key,
                    blob_id integer not null references blob(id),
                    provenance_id integer not null references provenance(id),
                    kind text,

                    constraint kind_valid check (kind in ('first_seen', 'last_seen')),
                    unique(blob_id, provenance_id, kind)
                ) strict;

                create table match
                -- This table is a representation of the matches found from scanning.
                --
                -- See the `noseyparker::match_type::Match` type in noseyparker for correspondence.
                (
                    blob_id integer not null references blob(id),

                    start_byte integer not null,
                    end_byte integer not null,

                    start_line integer not null,
                    start_column integer not null,

                    end_line integer not null,
                    end_column integer not null,

                    before_snippet blob not null,
                    matching_input blob not null,
                    after_snippet blob not null,

                    group_index integer not null,
                    group_input blob not null,

                    rule_name text not null,

                    -- NOTE: Checking just these fields ought to be sufficient to ensure that
                    --       entries in the table are unique.
                    unique (
                        blob_id,
                        start_byte,
                        end_byte,
                        group_index,
                        rule_name
                    )
                ) strict;

                -- An index to allow quick grouping of equivalent matches
                create index match_grouping_index on match (group_input, rule_name);
            "#})?;
            set_user_version(new_user_version)?;
        }

        // -----------------------------------------------------------------------------------------
        // migration 4
        //
        // Add an explicit `id` primary key to the match table.
        //
        // Sqlite doesn't allow adding a primary key via `alter table`, so we do a roundabout
        // migration by renaming, creating a new table, copying, an dropping the old one.
        // -----------------------------------------------------------------------------------------
        let user_version = get_user_version()?;
        if user_version == 3 {
            let new_user_version = 4;
            debug!("Migrating database schema from version {user_version} to {new_user_version}");
            tx.execute_batch(indoc! {r#"
                alter table match rename to old_match;

                create table match
                -- This table is a representation of the matches found from scanning.
                --
                -- See the `noseyparker::match_type::Match` type in noseyparker for correspondence.
                (
                    id integer primary key,
                    blob_id integer not null references blob(id),

                    start_byte integer not null,
                    end_byte integer not null,

                    start_line integer not null,
                    start_column integer not null,

                    end_line integer not null,
                    end_column integer not null,

                    before_snippet blob not null,
                    matching_input blob not null,
                    after_snippet blob not null,

                    group_index integer not null,
                    group_input blob not null,

                    rule_name text not null,

                    -- NOTE: Checking just these fields ought to be sufficient to ensure that
                    --       entries in the table are unique.
                    unique (
                        blob_id,
                        start_byte,
                        end_byte,
                        group_index,
                        rule_name
                    )
                ) strict;

                insert into match (
                    blob_id,
                    start_byte,
                    end_byte,
                    start_line,
                    start_column,
                    end_line,
                    end_column,
                    before_snippet,
                    matching_input,
                    after_snippet,
                    group_index,
                    group_input,
                    rule_name
                ) select * from old_match;

                drop table old_match;

                -- An index to allow quick grouping of equivalent matches
                create index match_grouping_index on match (group_input, rule_name);

            "#})?;
            set_user_version(new_user_version)?;
        }

        // -----------------------------------------------------------------------------------------
        // migration 5
        //
        // Add a couple tables for annotating findings: commenting and accept/reject status.
        // These tables are not currently populated by open-source Nosey Parker.
        // -----------------------------------------------------------------------------------------
        let user_version = get_user_version()?;
        if user_version == 4 {
            let new_user_version = 5;
            debug!("Migrating database schema from version {user_version} to {new_user_version}");

            // determine if a finding_status table already exists in the datastore
            let finding_status_exists: bool = tx.query_row(indoc! {r#"
                select exists (
                    select *
                    from sqlite_master
                    where type = 'table'
                    and name = 'finding_status'
                )
            "#}, (), |row| row.get(0))?;

            // determine if a finding_comment table already exists in the datastore
            let finding_comment_exists: bool = tx.query_row(indoc! {r#"
                select exists (
                    select *
                    from sqlite_master
                    where type = 'table'
                    and name = 'finding_comment'
                )
            "#}, (), |row| row.get(0))?;

            if finding_status_exists {
                tx.execute("alter table finding_status rename to old_finding_status", ())?;
            }

            if finding_comment_exists {
                tx.execute("alter table finding_comment rename to old_finding_comment", ())?;
            }

            tx.execute_batch(indoc! {r#"
                create table finding_status
                -- This table records the accepted/rejected status of findings.
                (
                    id integer primary key,
                    rule_name text not null,
                    group_input blob not null,
                    status text not null,
                    constraint status_valid check (status in ('accept', 'reject')),
                    unique (rule_name, group_input)
                ) strict;

                create table if not exists finding_comment
                -- This table records ad-hoc comments assigned to findings.
                (
                    id integer primary key,
                    rule_name text not null,
                    group_input blob not null,
                    comment text not null,
                    unique (rule_name, group_input),
                    constraint comment_valid check (comment != '')
                ) strict;
            "#})?;

            // migrate old finding status data
            if finding_status_exists {
                tx.execute_batch(indoc! {r#"
                    insert into finding_status (rule_name, group_input, status)
                    select rule_name, group_input, status
                    from old_finding_status;

                    drop table old_finding_status;
                "#})?;
            }

            // migrate old finding comment data
            if finding_comment_exists {
                tx.execute_batch(indoc! {r#"
                    insert into finding_comment (rule_name, group_input, comment)
                    select rule_name, group_input, comment
                    from old_finding_comment;

                    drop table old_finding_comment;
                "#})?;
            }

            set_user_version(new_user_version)?;
        }

        assert_eq!(get_user_version()?, 5);
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
// MatchGroupMetadata
// -------------------------------------------------------------------------------------------------

/// Metadata for a group of matches that have identical rule name and match content.
#[derive(Debug, Serialize)]
pub struct MatchGroupMetadata {
    /// The name of the rule of all the matches in the group
    pub rule_name: String,

    /// The matched content of all the matches in the group
    #[serde(with = "BStringSerde")]
    pub match_content: BString,

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
