use anyhow::{bail, Context, Result};
use bstr::{BStr, ByteSlice};
use indenter::indented;
use serde::Serialize;
use std::fmt::{Display, Formatter, Write};

use noseyparker::blob_metadata::BlobMetadata;
use noseyparker::bstring_escape::Escaped;
use noseyparker::datastore::{Datastore, MatchGroupMetadata, MatchId, Status};
use noseyparker::defaults::get_builtin_rules;
use noseyparker::digest::sha1_hexdigest;
use noseyparker::match_type::Match;
use noseyparker::provenance::Provenance;
use noseyparker::provenance_set::ProvenanceSet;

use crate::args::{GlobalArgs, ReportArgs, ReportOutputFormat};
use crate::reportable::Reportable;

mod human_format;
mod sarif_format;
mod styles;

use styles::{Styles, StyledObject};

pub fn run(global_args: &GlobalArgs, args: &ReportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;
    let output = args
        .output_args
        .get_writer()
        .context("Failed to get output writer")?;

    let max_matches = if args.max_matches <= 0 {
        None
    } else {
        Some(args.max_matches.try_into().unwrap())
    };

    // enable output styling:
    // - if the output destination is not explicitly specified and colors are not disabled
    // - if the output destination *is* explicitly specified and colors are forced on
    let styles_enabled = if args.output_args.output.is_none() {
        global_args.use_color(std::io::stdout())
    } else {
        global_args.color == crate::args::Mode::Always
    };

    let styles = Styles::new(styles_enabled);

    let reporter = DetailsReporter {
        datastore,
        max_matches,
        styles,
    };
    reporter.report(args.output_args.format, output)
}

struct DetailsReporter {
    datastore: Datastore,
    max_matches: Option<usize>,
    styles: Styles,
}

impl DetailsReporter {
    fn get_matches(&self, metadata: &MatchGroupMetadata) -> Result<Vec<ReportMatch>> {
        Ok(self
            .datastore
            .get_match_group_data(metadata, self.max_matches)
            .with_context(|| format!("Failed to get match data for group {metadata:?}"))?
            .into_iter()
            .map(|(p, md, id, m)| ReportMatch { ps: p, md, id, m })
            .collect())
    }

    fn style_finding_heading<D>(&self, val: D) -> StyledObject<D> {
        self.styles.style_finding_heading.apply_to(val)
    }

    fn style_rule<D>(&self, val: D) -> StyledObject<D> {
        self.styles.style_rule.apply_to(val)
    }

    fn style_heading<D>(&self, val: D) -> StyledObject<D> {
        self.styles.style_heading.apply_to(val)
    }

    fn style_match<D>(&self, val: D) -> StyledObject<D> {
        self.styles.style_match.apply_to(val)
    }

    fn style_metadata<D>(&self, val: D) -> StyledObject<D> {
        self.styles.style_metadata.apply_to(val)
    }
}

impl Reportable for DetailsReporter {
    type Format = ReportOutputFormat;

    fn report<W: std::io::Write>(&self, format: Self::Format, writer: W) -> Result<()> {
        match format {
            ReportOutputFormat::Human => self.human_format(writer),
            ReportOutputFormat::Json => self.json_format(writer),
            ReportOutputFormat::Jsonl => self.jsonl_format(writer),
            ReportOutputFormat::Sarif => self.sarif_format(writer),
        }
    }
}

impl DetailsReporter {
    /// Write findings in JSON-like format to `writer`.
    ///
    /// If `begin` is supplied, it is written before any finding is.
    /// If `sep` is supplied, it is written to separate each finding.
    /// If `end` is suplied, it is written after all findings have been.
    ///
    /// This is flexible enough to express both JSON and JSONL output formats, and to do so without
    /// having to accumulate all the findings into memory.
    fn write_json_findings<W: std::io::Write>(
        &self,
        mut writer: W,
        begin: Option<&str>,
        sep: Option<&str>,
        end: Option<&str>,
    ) -> Result<()> {
        let datastore = &self.datastore;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        if let Some(begin) = begin {
            write!(writer, "{}", begin)?;
        }

        let mut first = true;

        for metadata in group_metadata {
            if !first {
                if let Some(sep) = sep {
                    write!(writer, "{}", sep)?;
                }
            }
            first = false;

            let matches = self.get_matches(&metadata)?;
            let f = Finding::MatchGroup(MatchGroup::new(metadata, matches));
            serde_json::to_writer(&mut writer, &f)?;
        }

        if let Some(end) = end {
            write!(writer, "{}", end)?;
        }

        Ok(())
    }

    fn json_format<W: std::io::Write>(&self, writer: W) -> Result<()> {
        self.write_json_findings(writer, Some("[\n"), Some(",\n"), Some("\n]"))
    }

    fn jsonl_format<W: std::io::Write>(&self, writer: W) -> Result<()> {
        self.write_json_findings(writer, None, Some("\n"), Some("\n"))
    }

}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Finding {
    /// A group of matches that all have the same rule and capture group content
    #[serde(rename = "finding")]
    MatchGroup(MatchGroup),
}

#[derive(Serialize)]
struct MatchGroup {
    #[serde(flatten)]
    metadata: MatchGroupMetadata,
    matches: Vec<ReportMatch>,
}

#[derive(Serialize)]
struct ReportMatch {
    #[serde(rename = "provenance")]
    ps: ProvenanceSet,

    #[serde(rename = "blob_metadata")]
    md: BlobMetadata,

    #[serde(flatten)]
    m: Match,

    #[serde(skip)]
    #[allow(dead_code)]
    id: MatchId,
}

impl MatchGroup {
    fn new(metadata: MatchGroupMetadata, matches: Vec<ReportMatch>) -> Self {
        Self { metadata, matches }
    }

    fn rule_name(&self) -> &str {
        &self.metadata.rule_name
    }

    fn group_input(&self) -> &[u8] {
        &self.metadata.match_content
    }

    fn total_matches(&self) -> usize {
        self.metadata.num_matches
    }

    fn num_matches(&self) -> usize {
        self.matches.len()
    }
}
