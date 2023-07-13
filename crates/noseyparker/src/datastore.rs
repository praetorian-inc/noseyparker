use anyhow::{bail, Context, Result};
use bstr::BString;
use indoc::indoc;
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span};

use crate::blob_id::BlobId;
use crate::blob_metadata::BlobMetadata;
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
        let root_dir = root_dir.canonicalize()
            .with_context(|| format!("Failed to canonicalize datastore path at {}", root_dir.display()))?;
        let mut ds = Self {
            root_dir,
            conn,
        };
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

    /// Record the given blob metadata into the datastore.
    ///
    /// The given entries are recorded in a single transaction.
    pub fn record_blob_metadata<'a, T: IntoIterator<Item = &'a BlobMetadata>>(
        &mut self,
        blob_metadata: T,
    ) -> Result<()> {
        let _span = debug_span!("Datastore::record_blob_metadata", "{}", self.root_dir.display()).entered();

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(indoc! {r#"
                insert or replace into blob_metadata(blob_id, size, mime_essence, charset)
                values (?, ?, ?, ?)
            "#})?;

            for md in blob_metadata {
                stmt.execute((&md.id.hex(), md.num_bytes(), md.mime_essence(), md.charset()))?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Record the given matches into the datastore.
    ///
    /// The given entries are recorded in a single transaction.
    pub fn record_matches<'a, T: IntoIterator<Item = &'a Match>>(
        &mut self,
        matches: T,
    ) -> Result<usize> {
        let _span = debug_span!("Datastore::record_matches", "{}", self.root_dir.display()).entered();

        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare_cached(indoc! {r#"
            insert or replace into matches(
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
                rule_name,
                provenance_type,
                provenance
            ) values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#})?;
        let mut num_changed = 0;
        for m in matches {
            let span = &m.location.offset_span;
            let src = &m.location.source_span;
            let (ptype, ppath) = match &m.provenance {
                Provenance::File { path } => ("file", path.to_string_lossy()),
                Provenance::GitRepo { path } => ("git", path.to_string_lossy()),
            };
            // FIXME: the number of changed rows is not the number of newly found matches!
            num_changed += stmt.execute((
                m.blob_id.hex(),
                span.start,
                span.end,
                src.start.line,
                src.start.column,
                src.end.line,
                src.end.column,
                m.snippet.before.as_slice(),
                m.snippet.matching.as_slice(),
                m.snippet.after.as_slice(),
                &m.capture_group_index,
                m.match_content.as_slice(),
                &m.rule_name,
                ptype,
                ppath,
            ))?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(num_changed)
    }

    /// Summarize all recorded findings.
    pub fn summarize(&self) -> Result<MatchSummary> {
        let _span = debug_span!("Datastore::summarize", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select rule_name, count(*) grouped_count, sum(num_matches) total_count
            from (
                select group_input, rule_name, count(*) num_matches
                from matches
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

    /// Get the root directory that contains this `Datastore`.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Get metadata for all groups of identical matches recorded within this `Datastore`.
    pub fn get_match_group_metadata(&self) -> Result<Vec<MatchGroupMetadata>> {
        let _span =
            debug_span!("Datastore::get_match_group_metadata", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select group_input, rule_name, count(*)
            from matches
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
    ) -> Result<Vec<(Option<BlobMetadata>, Match)>> {
        let _span = debug_span!("Datastore::get_match_group_data", "{}", self.root_dir.display()).entered();

        let mut stmt = self.conn.prepare_cached(indoc! {r#"
            select
                m.blob_id,
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
                m.provenance_type,
                m.provenance,

                b.size,
                b.mime_essence,
                b.charset
            from matches m
            left outer join blob_metadata b on (m.blob_id = b.blob_id)
            where m.rule_name = ? and m.group_input = ?
            order by m.blob_id, m.start_byte, m.end_byte
            limit ?
        "#})?;

        let limit: i64 = match limit {
            Some(limit) => limit.try_into().expect("limit should be convertible"),
            None => -1,
        };
        let entries = stmt.query_map((&metadata.rule_name, metadata.match_content.as_slice(), limit), |row| {
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
                provenance: provenance_from_parts(row.get(11)?, row.get(12)?)
                    .expect("provenance value from database should be valid"),
            };
            let num_bytes: Option<usize> = row.get(13)?;
            let mime_essence: Option<String> = row.get(14)?;
            let charset: Option<String> = row.get(15)?;
            let b = num_bytes.map(|num_bytes| {
                BlobMetadata {
                    id: blob_id,
                    num_bytes,
                    mime_essence,
                    charset,
                }
            });
            Ok((b, m))
        })?;
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
        // migration 1
        // -----------------------------------------------------------------------------------------
        let user_version: u64 = get_user_version()?;
        if user_version == 0 {
            let new_user_version = user_version + 1;
            debug!(
                "Migrating database schema from version {} to {}",
                user_version, new_user_version
            );
            tx.execute_batch(indoc! {r#"
                create table matches
                -- This table is a fully denormalized representation of the matches found from
                -- scanning.
                --
                -- See the `Match` type in noseyparker for correspondence.
                --
                -- Eventually we should refine the database schema, normalizing where appropriate.
                -- Doing so could allow for better write performance and smaller databases.
                (
                    blob_id text not null,

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

                    provenance_type text not null,
                    provenance blob not null,

                    -- NOTE: We really want this entire table to have unique values.
                    --       But checking just these fields ought to be sufficient to ensure that;
                    --       the remaining fields are either derived from these or are not relevant
                    --       to match deduping (like provenance).
                    --       Checking fewer fields should be cheaper than checking _all_ fields.
                    unique (
                        blob_id,
                        start_byte,
                        end_byte,
                        group_index,
                        rule_name
                    )
                );

                -- An index to allow quick grouping of equivalent matches
                create index matches_grouping_index on matches (group_input, rule_name);
            "#})?;
            set_user_version(new_user_version)?;
        }

        // -----------------------------------------------------------------------------------------
        // migration 2
        // -----------------------------------------------------------------------------------------
        let user_version: u64 = get_user_version()?;
        if user_version == 1 {
            let new_user_version = user_version + 1;
            debug!(
                "Migrating database schema from version {} to {}",
                user_version, new_user_version
            );

            tx.execute_batch(indoc! {r#"
                create table blob_metadata
                -- This table records various bits of metadata about blobs.
                (
                    blob_id text primary key,
                    size integer not null,
                    mime_essence text,
                    charset text,

                    constraint valid_blob_id check(
                        length(blob_id) == 40 and not glob('*[^abcdefABCDEF1234567890]*', blob_id)
                    ),
                    constraint valid_size check(0 <= size)
                );
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
fn provenance_from_parts(tag: String, path: String) -> Result<Provenance> {
    match tag.as_str() {
        "git" => Ok(Provenance::GitRepo {
            path: PathBuf::from(path),
        }),
        "file" => Ok(Provenance::File {
            path: PathBuf::from(path),
        }),
        t => bail!("Provenance tag {:?} is invalid", t),
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
    #[serde(with="crate::utils::BStringSerde")]
    pub match_content: BString,

    /// The number of matches in the group
    pub num_matches: usize,
}
