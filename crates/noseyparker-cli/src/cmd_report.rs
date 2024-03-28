use anyhow::{bail, Context, Result};
use bstr::{BStr, ByteSlice};
use indenter::indented;
use schemars::JsonSchema;
use serde::Serialize;
use std::fmt::{Display, Formatter, Write};

use noseyparker::blob_metadata::BlobMetadata;
use noseyparker::bstring_escape::Escaped;
use noseyparker::datastore::{Datastore, FindingDataEntry, FindingMetadata, Status};
use noseyparker::defaults::get_builtin_rules;
use noseyparker::match_type::{Group, Groups, Match};
use noseyparker::provenance::Provenance;
use noseyparker::provenance_set::ProvenanceSet;

use crate::args::{FindingStatus, GlobalArgs, ReportArgs, ReportOutputFormat};
use crate::reportable::Reportable;

mod human_format;
mod sarif_format;
mod styles;

use styles::{StyledObject, Styles};

pub fn run(global_args: &GlobalArgs, args: &ReportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;
    let output = args
        .output_args
        .get_writer()
        .context("Failed to get output writer")?;

    let max_matches = if args.filter_args.max_matches <= 0 {
        None
    } else {
        Some(args.filter_args.max_matches.try_into().unwrap())
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
        finding_status: args.filter_args.finding_status,
        styles,
    };
    reporter.report(args.output_args.format, output)
}

struct DetailsReporter {
    datastore: Datastore,
    max_matches: Option<usize>,
    finding_status: Option<FindingStatus>,
    styles: Styles,
}

impl DetailsReporter {
    fn include_finding(&self, metadata: &FindingMetadata) -> bool {
        match self.finding_status {
            None => true,
            Some(status) => match (status, metadata.statuses.0.as_slice()) {
                (FindingStatus::Accept, &[Status::Accept]) => true,
                (FindingStatus::Reject, &[Status::Reject]) => true,
                (FindingStatus::Null, &[]) => true,
                (FindingStatus::Mixed, &[Status::Accept, Status::Reject]) => true,
                (FindingStatus::Mixed, &[Status::Reject, Status::Accept]) => true,
                _ => false,
            },
        }
    }

    fn get_finding_metadata(&self) -> Result<Vec<FindingMetadata>> {
        let datastore = &self.datastore;
        let mut group_metadata = datastore
            .get_finding_metadata()
            .context("Failed to get match group metadata from datastore")?;

        group_metadata.retain(|md| self.include_finding(&md));

        Ok(group_metadata)
    }

    fn get_matches(&self, metadata: &FindingMetadata) -> Result<Vec<ReportMatch>> {
        Ok(self
            .datastore
            .get_finding_data(metadata, self.max_matches)
            .with_context(|| format!("Failed to get matches for finding {metadata:?}"))
            .expect("should be able to find get matches for finding")
            .into_iter()
            .map(|e| e.into())
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
        let group_metadata = self.get_finding_metadata()?;

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
            let f = Finding::new(metadata, matches);
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

/// A group of matches that all have the same rule and capture group content
#[derive(Serialize, JsonSchema)]
pub(crate) struct Finding {
    #[serde(flatten)]
    metadata: FindingMetadata,
    matches: Vec<ReportMatch>,
}

/// A match produced by one of Nosey Parker's rules.
/// This corresponds to a single location.
#[derive(Serialize, JsonSchema)]
struct ReportMatch {
    provenance: ProvenanceSet,

    #[serde(rename = "blob_metadata")]
    blob_metadata: BlobMetadata,

    #[serde(flatten)]
    m: Match,

    /// An optional score assigned to the match
    #[validate(range(min = 0.0, max = 1.0))]
    score: Option<f64>,

    /// An optional comment assigned to the match
    comment: Option<String>,

    /// An optional status assigned to the match
    status: Option<Status>,
}

impl From<FindingDataEntry> for ReportMatch {
    fn from(e: FindingDataEntry) -> Self {
        ReportMatch {
            provenance: e.provenance,
            blob_metadata: e.blob_metadata,
            m: e.match_val,
            score: e.match_score,
            comment: e.match_comment,
            status: e.match_status,
        }
    }
}

impl Finding {
    fn new(metadata: FindingMetadata, matches: Vec<ReportMatch>) -> Self {
        Self { metadata, matches }
    }

    /// The name of the rule that produced this finding
    fn rule_name(&self) -> &str {
        &self.metadata.rule_name
    }

    fn groups(&self) -> &Groups {
        &self.metadata.groups
    }

    /// The total number of matches in this finding
    fn total_matches(&self) -> usize {
        self.metadata.num_matches
    }

    /// The number of matches present in this finding
    fn num_matches_available(&self) -> usize {
        self.matches.len()
    }
}
