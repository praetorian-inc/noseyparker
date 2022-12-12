use anyhow::{Context, Result};
use indenter::indented;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Write};

use noseyparker::datastore::{Datastore, MatchGroupMetadata};
use noseyparker::match_type::Match;
use noseyparker::provenance::Provenance;
use noseyparker::utils::decode_utf8_lossy_escape;

use crate::args;

pub fn run(_global_args: &args::GlobalArgs, args: &args::ReportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore)?;
    let mut writer = args
        .output_args
        .get_writer()
        .context("Failed to open output destination for writing")?;
    let group_metadata = datastore
        .get_match_group_metadata()
        .context("Failed to get match group metadata from datastore")?;

    match &args.output_args.format {
        args::OutputFormat::Human => {
            let num_findings = group_metadata.len();
            for (finding_num, metadata) in group_metadata.into_iter().enumerate() {
                let finding_num = finding_num + 1;
                let matches = datastore
                    .get_match_group_matches(&metadata, Some(3))
                    .with_context(|| format!("Failed to get matches for group {:?}", metadata))?;
                let match_group = MatchGroup { metadata, matches };
                let res = writeln!(writer, "Finding {}/{}: {}", finding_num, num_findings, match_group);
                match res {
                    // Ignore SIGPIPE errors, like those that can come from piping to `head`
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => { return Ok(()) }
                    r => { r? }
                }
            }
            Ok(())
        }

        args::OutputFormat::Jsonl => {
            for metadata in group_metadata.into_iter() {
                let matches = datastore
                    .get_match_group_matches(&metadata, Some(3))
                    .with_context(|| format!("Failed to get matches for group {:?}", metadata))?;
                let match_group = MatchGroup { metadata, matches };

                let res: std::io::Result<()> = serde_json::to_writer(&mut writer, &match_group).map_err(|e| e.into());
                // .and_then(|()| writeln!(&mut writer));
                match res {
                    // Ignore SIGPIPE errors, like those that can come from piping to `head`
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => { return Ok(()) }
                    r => { r? }
                }
            }
            Ok(())
        }

        args::OutputFormat::Json => { panic!("unimplemented"); }
    }
}

/// A group of matches that all have the same rule and capture group content
#[derive(Serialize, Deserialize)]
struct MatchGroup {
    metadata: MatchGroupMetadata,
    matches: Vec<Match>,
}

impl MatchGroup {
    fn rule_name(&self) -> &str {
        &self.metadata.rule_name
    }

    fn group_input(&self) -> &[u8] {
        &self.metadata.group_input
    }

    fn total_matches(&self) -> usize {
        self.metadata.matches
    }

    fn num_matches(&self) -> usize {
        self.matches.len()
    }
}

impl Display for MatchGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.rule_name())?;

        let g = decode_utf8_lossy_escape(self.group_input());
        if g.contains('\n') {
            writeln!(f, "Match:\n")?;
            writeln!(indented(f).with_str("    "), "{}", g)?;
            writeln!(f)?;
        } else {
            writeln!(f, "Match: {}", g)?;
        }

        writeln!(f, "Showing {}/{} occurrences:", self.num_matches(), self.total_matches())?;
        writeln!(f)?;

        let mut f = indented(f).with_str("    ");
        for (i, m) in self.matches.iter().enumerate() {
            let i = i + 1;
            writeln!(f, "Occurrence {}:", i)?;
            match &m.provenance {
                Provenance::FromFile(p) => {
                    writeln!(f, "File: {}", p.to_string_lossy())?;
                }
                Provenance::FromGitRepo(p) => {
                    writeln!(f, "Git repo: {}", p.to_string_lossy())?;
                    writeln!(f, "Blob: {}", &m.blob_id)?;
                }
            }
            writeln!(f, "Lines: {}", &m.matching_input_source_span)?;
            writeln!(f)?;
            writeln!(indented(&mut f).with_str("    "), "{}", m.snippet())?;
            writeln!(f)?;
        }

        Ok(())
    }
}
