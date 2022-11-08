use anyhow::{Context, Result, bail};
use indoc::indoc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span};

use crate::blob_id::BlobId;
use crate::location::{OffsetSpan, SourceSpan, SourcePoint};
use crate::match_type::Match;
use crate::provenance::Provenance;

// -------------------------------------------------------------------------------------------------
// Datastore
// -------------------------------------------------------------------------------------------------
pub struct Datastore {
    /// The root directory of everything contained in this `Datastore`.
    root_dir: PathBuf,

    /// A connection to the database backing this `Datastore`.
    conn: Connection,
}

impl Datastore {
    /// Create a new datastore at `root_dir` if one does not exist,
    /// or open an existing one if present.
    pub fn create_or_open(root_dir: &Path) -> Result<Self> {
        Self::create(root_dir)
            .or_else(|_e| Self::open(root_dir))
            .with_context(|| format!("Failed to open datastore at {:?}", root_dir))
    }

    /// Open the existing datastore at `root_dir`.
    pub fn open(root_dir: &Path) -> Result<Self> {
        let db_path = root_dir.join("datastore.db");
        let conn = Self::new_connection(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;
        let mut ds = Self {
            root_dir: root_dir.to_owned(),
            conn,
        };
        ds.migrate()
            .with_context(|| format!("Failed to migrate database at {:?}", db_path))?;
        Ok(ds)
    }

    /// Create a new datastore at `root_dir` and open it.
    pub fn create(root_dir: &Path) -> Result<Self> {
        // Create datastore directory
        std::fs::create_dir(root_dir).with_context(|| {
            format!("Failed to create datastore root directory at {:?}", root_dir)
        })?;

        // Generate .gitignore file
        std::fs::write(&root_dir.join(".gitignore"), "*\n").with_context(|| {
            format!("Failed to write .gitignore to datastore at {:?}", root_dir)
        })?;

        Self::open(root_dir)
    }

    pub fn tmpdir(&self) -> PathBuf {
        self.root_dir.join("scratch")
    }

    fn new_connection(path: &Path) -> Result<Connection> {
        let conn = Connection::open(&path)?;

        conn.pragma_update(None, "journal_mode", "wal")?; // https://www.sqlite.org/wal.html
        conn.pragma_update(None, "foreign_keys", "on")?; // https://sqlite.org/foreignkeys.html
        conn.pragma_update(None, "synchronous", "normal")?; // https://sqlite.org/pragma.html#pragma_synchronous
                                                            //
        let limit: i64 = -512 * 1024; // 512MiB limit
        conn.pragma_update(None, "cache_size", limit)?; // https://sqlite.org/pragma.html#pragma_cache_size

        Ok(conn)
    }

    fn migrate(&mut self) -> Result<u64> {
        let _span = debug_span!("Datastore::migrate", "{:?}", self.root_dir).entered();
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
                -- See the `noseyparker::match::Match` type for correspondence.
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
            tx.commit()?;
        }
        Ok(user_version)
    }

    pub fn analyze(&self) -> Result<()> {
        self.conn.execute("analyze", [])?;
        Ok(())
    }

    pub fn record_matches<'a, T: IntoIterator<Item = &'a Match>>(
        &mut self,
        matches: T,
    ) -> Result<usize> {
        let _span = debug_span!("Datastore::record_matches", "{:?}", self.root_dir).entered();

        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare_cached(indoc! {r#"
            insert into matches(
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
            on conflict do nothing
        "#})?;
        let mut num_changed = 0;
        for m in matches {
            let span = &m.matching_input_offset_span;
            let src = &m.matching_input_source_span;
            let (ptype, ppath) = match &m.provenance {
                Provenance::FromFile(p) => ("file", p.to_string_lossy()),
                Provenance::FromGitRepo(p) => ("git", p.to_string_lossy()),
            };
            num_changed += stmt.execute((
                m.blob_id.hex(),
                span.start,
                span.end,
                src.start.line,
                src.start.column,
                src.end.line,
                src.end.column,
                &m.before_snippet,
                &m.matching_input,
                &m.after_snippet,
                &m.group_index,
                &m.group_input,
                &m.rule_name,
                ptype,
                ppath,
            ))?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(num_changed)
    }

    pub fn summarize(&self) -> Result<MatchSummary> {
        let _span = debug_span!("Datastore::summarize", "{:?}", self.root_dir).entered();

        let mut stmt = self.conn.prepare_cached(indoc!{r#"
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
                grouped_count: row.get(1)?,
                total_count: row.get(2)?,
            })
        })?;
        let mut es = Vec::new();
        for e in entries.into_iter() {
            es.push(e?);
        }
        Ok(MatchSummary(es))
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn get_match_group_metadata(&self) -> Result<Vec<MatchGroupMetadata>> {
        let _span = debug_span!("Datastore::get_match_group_metadata", "{:?}", self.root_dir).entered();

        let mut stmt = self.conn.prepare_cached(indoc!{r#"
            select group_input, rule_name, count(*)
            from matches
            group by 1, 2
            order by 2
        "#})?;
        let entries = stmt.query_map((), |row| {
            Ok(MatchGroupMetadata {
                group_input: row.get(0)?,
                rule_name: row.get(1)?,
                matches: row.get(2)?,
            })
        })?;
        let mut es = Vec::new();
        for e in entries.into_iter() {
            es.push(e?);
        }
        Ok(es)
    }

    pub fn get_match_group_matches(&self, metadata: &MatchGroupMetadata, limit: Option<usize>) -> Result<Vec<Match>> {
        let _span = debug_span!("Datastore::match_groups", "{:?}", self.root_dir).entered();

        let mut stmt = self.conn.prepare_cached(indoc!{r#"
            select
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
                provenance_type,
                provenance
            from matches
            where rule_name = ? and group_input = ?
            order by blob_id, start_byte, end_byte
            limit ?
        "#})?;

        let limit: i64 = match limit {
            Some(limit) => limit.try_into().expect("limit should be convertible"),
            None => -1,
        };
        let entries = stmt.query_map((&metadata.rule_name, &metadata.group_input, limit), |row| {
            let v0: String = row.get(0)?;
            Ok(Match {
                blob_id: BlobId::from_hex(&v0).expect("blob id from database should be valid"),
                matching_input_offset_span: OffsetSpan {
                    start: row.get(1)?,
                    end: row.get(2)?,
                },
                matching_input_source_span: SourceSpan {
                    start: SourcePoint {
                        line: row.get(3)?,
                        column: row.get(4)?,
                    },
                    end: SourcePoint {
                        line: row.get(5)?,
                        column: row.get(6)?,
                    },
                },
                before_snippet: row.get(7)?,
                matching_input: row.get(8)?,
                after_snippet: row.get(9)?,
                group_index: row.get(10)?,
                group_input: metadata.group_input.clone(),
                rule_name: metadata.rule_name.clone(),
                provenance: provenance_from_parts(row.get(11)?, row.get(12)?).expect("provenance value from database should be valid")
            })
        })?;
        let mut es = Vec::new();
        for e in entries.into_iter() {
            es.push(e?);
        }
        Ok(es)
    }
}


fn provenance_from_parts(tag: String, path: String) -> Result<Provenance> {
    match tag.as_str() {
        "git" => Ok(Provenance::FromGitRepo(PathBuf::from(path))),
        "file" => Ok(Provenance::FromFile(PathBuf::from(path))),
        t => bail!("Provenance tag {:?} is invalid", t),
    }
}

// -------------------------------------------------------------------------------------------------
// MatchSummary
// -------------------------------------------------------------------------------------------------
/// A summary of matches in a `Datastore`
#[derive(Deserialize, Serialize)]
pub struct MatchSummary(pub Vec<MatchSummaryEntry>);

#[derive(Deserialize, Serialize)]
pub struct MatchSummaryEntry {
    pub rule_name: String,
    pub grouped_count: usize,
    pub total_count: usize,
}

impl std::fmt::Display for MatchSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for entry in self.0.iter() {
            writeln!(f, "{}: {} ({})", entry.rule_name, entry.grouped_count, entry.total_count)?;
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// MatchGroupMetadata
// -------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct MatchGroupMetadata {
    pub rule_name: String,
    pub group_input: Vec<u8>,
    pub matches: usize,
}
