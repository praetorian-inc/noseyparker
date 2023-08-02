use anyhow::{bail, Context, Result};
use bstr::BString;
use indoc::indoc;
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span};

use crate::blob_id::BlobId;
use crate::blob_metadata::BlobMetadata;
use crate::git_commit_metadata::CommitMetadata;
use crate::git_url::GitUrl;
use crate::location::{Location, OffsetSpan, SourcePoint, SourceSpan};
use crate::match_type::Match;
use crate::provenance::Provenance;
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
    pub fn create_or_open(root_dir: &Path) -> Result<Self> {
        Self::create(root_dir)
            .or_else(|_e| Self::open(root_dir))
            .with_context(|| format!("Failed to open datastore at {}", root_dir.display()))
    }

    /// Open the existing datastore at `root_dir`.
    pub fn open(root_dir: &Path) -> Result<Self> {
        let db_path = root_dir.join("datastore.db");
        let conn = Self::new_connection(&db_path)
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

        Ok(ds)
    }

    /// Create a new datastore at `root_dir` and open it.
    pub fn create(root_dir: &Path) -> Result<Self> {
        // Create datastore directory
        std::fs::create_dir(root_dir).with_context(|| {
            format!("Failed to create datastore root directory at {}", root_dir.display())
        })?;

        // Generate .gitignore file
        std::fs::write(root_dir.join(".gitignore"), "*\n").with_context(|| {
            format!("Failed to write .gitignore to datastore at {}", root_dir.display())
        })?;

        Self::open(root_dir)
    }

    /// Get the path to this datastore's scratch directory.
    pub fn scratch_dir(&self) -> PathBuf {
        self.root_dir.join("scratch")
    }

    /// Get the path to this datastore's clones directory.
    pub fn clones_dir(&self) -> PathBuf {
        self.root_dir.join("clones")
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
        self.conn.execute("analyze", [])?;
        // self.conn.execute("pragma wal_checkpoint(truncate)", [])?;
        Ok(())
    }
}

// Public implementation, recording functions
impl Datastore {
    /// Record the given commit metadata into the datastore.
    ///
    /// The given entries are recorded in a single transaction.
    pub fn record_commit_metadata<T: IntoIterator<Item = CommitMetadata>>(
        &mut self,
        commit_metadata: T,
    ) -> Result<()> {
        panic!("FIXME: unimplemented");
    }

    /// Record the given provenance entries into the datastore.
    ///
    /// The given entries are recorded in a single transaction.
    pub fn record_blob_provenance<T: IntoIterator<Item = (BlobId, Provenance)>>(
        &mut self,
        blob_provenance: T,
    ) -> Result<()> {
        panic!("FIXME: unimplemented");

        /*
            let mut add_provenance_payload_file = tx.prepare_cached(indoc! {r#"
                insert or ignore into provenance_payload_file(path)
                values (?)
            "#})?;

            let mut add_provenance_payload_git = tx.prepare_cached(indoc! {r#"
                insert or ignore into provenance_payload_git(repo_path, commit_id, blob_path)
                values (?, ?, ?)
            "#})?;

            let mut add_provenance = tx.prepare_cached(indoc! {r#"
                insert or ignore into provenance(kind, payload_id)
                values (?, ?)
            "#})?;
        */
    }

    /// Record the given blob metadata entries and matches into the datastore.
    /// Returns the number of matches added.
    ///
    /// The given entries are recorded in a single transaction.
    pub fn record_metadata_and_matches<I1, I2>(
        &mut self,
        blob_metadata: I1,
        matches: I2,
    ) -> Result<u64>
    where
        I1: IntoIterator<Item = BlobMetadata>,
        I2: IntoIterator<Item = Match>,
    {
        let _span =
            debug_span!("Datastore::record_metadata_and_matches", "{}", self.root_dir.display())
                .entered();

        let tx = self.conn.transaction()?;

        let num_added = {
            let mut add_blob = tx.prepare_cached(indoc! {r#"
                insert into blob(blob_id, size, mime_essence, charset)
                values (?, ?, ?, ?)
                on conflict do update set
                    mime_essence = excluded.mime_essence,
                    size = excluded.size,
                    charset = excluded.charset
            "#})?;

            let mut get_blob_id = tx.prepare_cached(indoc! {r#"
                select id from blob where blob_id = ?
            "#})?;

            let mut contains_match = tx.prepare_cached(indoc! {r#"
                select * from match
                where
                    blob_id = ? and
                    start_byte = ? and
                    end_byte = ? and
                    group_index = ? and
                    rule_name = ?
                limit 1
            "#})?;

            let mut add_match = tx.prepare_cached(indoc! {r#"
                insert or replace into match(
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
                ) values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#})?;

            for md in blob_metadata {
                add_blob
                    .execute((md.id.hex(), md.num_bytes(), md.mime_essence(), md.charset()))
                    .context("Failed to add metadata")?;
            }

            let mut num_added = 0;
            for m in matches {
                let span = &m.location.offset_span;
                let src = &m.location.source_span;
                let rule_name = &m.rule_name;

                let blob_id: i64 = get_blob_id
                    .query_row((m.blob_id.hex(),), |row| row.get(0))
                    .context("Failed to get blob id")?;

                if !contains_match
                    .exists((&blob_id, span.start, span.end, m.capture_group_index, rule_name))
                    .context("Failed to check if match exists")?
                {
                    num_added += 1;
                }

                add_match
                    .execute((
                        blob_id,
                        span.start,
                        span.end,
                        src.start.line,
                        src.start.column,
                        src.end.line,
                        src.end.column,
                        m.snippet.before.as_slice(),
                        m.snippet.matching.as_slice(),
                        m.snippet.after.as_slice(),
                        m.capture_group_index,
                        m.match_content.as_slice(),
                        rule_name,
                    ))
                    .context("Failed to add match")?;
            }
            num_added
        };

        tx.commit()?;
        Ok(num_added)
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
            select group_input, rule_name, count(*)
            from match
            group by 1, 2
            order by 2
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(MatchGroupMetadata {
                match_content: BString::new(row.get(0)?),
                rule_name: row.get(1)?,
                num_matches: row.get(2)?,
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
    ) -> Result<Vec<(BlobMetadata, Match)>> {
        let _span =
            debug_span!("Datastore::get_match_group_data", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
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
                b.charset
            from match m
            inner join blob b on (m.blob_id = b.id)
            where m.group_input = ?1 and m.rule_name = ?2
            order by m.blob_id, m.start_byte, m.end_byte
            limit ?3
        "#})?;

        let limit: i64 = match limit {
            Some(limit) => limit.try_into().expect("limit should be convertible"),
            None => -1,
        };
        let entries = stmt.query_map(
            (metadata.match_content.as_slice(), &metadata.rule_name, limit),
            |row| {
                let v0: String = row.get(0)?;
                let blob_id = BlobId::from_hex(&v0).expect("blob id from database should be valid");
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
                Ok((b, m))
            },
        )?;
        let mut es = Vec::new();
        for e in entries {
            es.push(e?);
        }
        Ok(es)
    }
}

// Private implementation
impl Datastore {
    fn new_connection(path: &Path) -> Result<Connection> {
        let conn = Connection::open(path)?;

        conn.pragma_update(None, "journal_mode", "wal")?; // https://www.sqlite.org/wal.html
        conn.pragma_update(None, "foreign_keys", "on")?; // https://sqlite.org/foreignkeys.html
        conn.pragma_update(None, "synchronous", "normal")?; // https://sqlite.org/pragma.html#pragma_synchronous

        // FIXME: make this a command-line parameter
        let limit: i64 = -8 * 1024 * 1024; // 8GiB limit
        conn.pragma_update(None, "cache_size", limit)?; // https://sqlite.org/pragma.html#pragma_cache_size

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

        // -----------------------------------------------------------------------------------------
        // migration 3
        // -----------------------------------------------------------------------------------------
        let user_version: u64 = get_user_version()?;
        if user_version == 1 || user_version == 2 {
            bail!(
                "Datastores from earlier Nosey Parker versions cannot be migrated to \
                  the new format; rescanning the inputs with a new datastore will be required."
            );
        }
        if user_version > 3 {
            bail!("Unknown schema version {user_version}");
        }

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

                    -- The commit timestamp
                    commit_date text not null,

                    -- The committer
                    committer text not null,

                    -- The commit author timestamp
                    author_date text not null,

                    -- The commit author
                    author text not null,

                    -- The commit message
                    message text not null,

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

                create table provenance_payload_git
                -- This table records provenance information about Git commits.
                (
                    id integer primary key,

                    -- The filesystem path of the Git repo
                    repo_path text not null,

                    commit_id integer references git_commit(id),

                    -- The path of the blob within the commit
                    blob_path blob not null,

                    unique(commit_id, blob_path)
                ) strict;

                create table provenance
                -- This table encodes a union of the `provenance_payload_*` tables.
                (
                    id integer primary key,

                    kind text not null,
                    -- The ID of the provenance payload;
                    -- references one of the `provenance_payload_*` tables depending on `kind`
                    payload_id integer not null,

                    unique(kind, payload_id),

                    constraint valid_kind check(kind in ('file', 'git'))
                ) strict;

                create table blob_provenance
                -- This table records the various ways in which a blob was encountered.
                (
                    id integer primary key,
                    blob_id integer not null references blob(id),
                    provenance_id integer not null references provenance(id),
                    unique(blob_id, provenance_id)
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

/// Metadata for a group of matches that have identical match content.
#[derive(Debug, Serialize)]
pub struct MatchGroupMetadata {
    /// The name of the rule of all the matches in the group
    pub rule_name: String,

    /// The matched content of all the matches in the group
    #[serde(with = "crate::utils::BStringSerde")]
    pub match_content: BString,

    /// The number of matches in the group
    pub num_matches: usize,
}
