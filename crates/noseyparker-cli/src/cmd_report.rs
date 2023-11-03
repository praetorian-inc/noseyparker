use anyhow::{bail, Context, Result};
use bstr::{BStr, ByteSlice};
use indenter::indented;
use lazy_static::lazy_static;
use serde::Serialize;
use serde_sarif::sarif;
use std::fmt::{Display, Formatter, Write};
use tracing::debug;

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

pub fn run(global_args: &GlobalArgs, args: &ReportArgs) -> Result<()> {
    debug!("Args:\n{global_args:#?}\n{args:#?}");

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
    let reporter = DetailsReporter {
        datastore,
        max_matches,
    };
    reporter.report(args.output_args.format, output)
}

struct DetailsReporter {
    datastore: Datastore,
    max_matches: Option<usize>,
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
    fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore = &self.datastore;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        let num_findings = group_metadata.len();
        for (finding_num, metadata) in group_metadata.into_iter().enumerate() {
            let finding_num = finding_num + 1;
            let matches = self.get_matches(&metadata)?;
            let match_group = MatchGroup { metadata, matches };
            writeln!(
                &mut writer,
                "{} {}",
                STYLE_FINDING_HEADING.apply_to(format!("Finding {finding_num}/{num_findings}:")),
                match_group,
            )?;
        }
        Ok(())
    }

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

    fn make_sarif_result(&self, finding: &MatchGroup) -> Result<sarif::Result> {
        let matches = &finding.matches;
        let metadata = &finding.metadata;

        let first_match_blob_id = match matches.first() {
            Some(entry) => entry.m.blob_id.to_string(),
            None => bail!("Failed to get group match data for group {metadata:?}"),
        };
        let message = sarif::MessageBuilder::default()
            .text(format!(
                "Rule {:?} found {} {}.\nFirst blob id matched: {}",
                metadata.rule_name,
                metadata.num_matches,
                if metadata.num_matches == 1 {
                    "match".to_string()
                } else {
                    "matches".to_string()
                },
                first_match_blob_id,
            ))
            .build()?;

        // Will store every match location for the runs.results.location array property
        let locations: Vec<sarif::Location> = matches
            .iter()
            .flat_map(|m| {
                let ReportMatch { ps, md, m, .. } = m;
                ps.iter().map(move |p| {
                    let source_span = &m.location.source_span;
                    // let offset_span = &m.location.offset_span;

                    let mut additional_properties =
                        vec![(String::from("blob_metadata"), serde_json::json!(md))];

                    let uri = match p {
                        Provenance::File(e) => e.path.to_string_lossy().into_owned(),
                        Provenance::GitRepo(e) => {
                            if let Some(p) = &e.commit_provenance {
                                additional_properties.push((
                                    String::from("commit_provenance"),
                                    serde_json::json!(p),
                                ));
                            }
                            e.repo_path.to_string_lossy().into_owned()
                        }
                    };

                    let additional_properties =
                        std::collections::BTreeMap::from_iter(additional_properties);
                    let properties = sarif::PropertyBagBuilder::default()
                        .additional_properties(additional_properties)
                        .build()?;

                    let location = sarif::LocationBuilder::default()
                        .physical_location(
                            sarif::PhysicalLocationBuilder::default()
                                .artifact_location(
                                    sarif::ArtifactLocationBuilder::default().uri(uri).build()?,
                                )
                                // .context_region() FIXME: fill this in with location info of surrounding context
                                .region(
                                    sarif::RegionBuilder::default()
                                        .start_line(source_span.start.line as i64)
                                        .start_column(source_span.start.column as i64)
                                        .end_line(source_span.end.line as i64)
                                        .end_column(source_span.end.column as i64 + 1)
                                        // FIXME: including byte offsets seems to confuse VSCode SARIF Viewer. Why?
                                        /*
                                        .byte_offset(offset_span.start as i64)
                                        .byte_length(offset_span.len() as i64)
                                        */
                                        .snippet(
                                            sarif::ArtifactContentBuilder::default()
                                                .text(m.snippet.matching.to_string())
                                                .build()?,
                                        )
                                        .build()?,
                                )
                                .build()?,
                        )
                        .logical_locations([sarif::LogicalLocationBuilder::default()
                            .kind("blob")
                            .name(m.blob_id.to_string())
                            .properties(properties)
                            .build()?])
                        .build()?;
                    Ok(location)
                })
            })
            .collect::<Result<_>>()?;

        let sha1_fingerprint = sha1_hexdigest(&metadata.match_content);

        // Build the result for the match
        let result = sarif::ResultBuilder::default()
            .rule_id(&metadata.rule_name)
            // .occurrence_count(locations.len() as i64)  // FIXME: enable?
            .message(message)
            .kind(sarif::ResultKind::Review.to_string())
            .locations(locations)
            .level(sarif::ResultLevel::Warning.to_string())
            .partial_fingerprints([("match_group_content/sha256/v1".to_string(), sha1_fingerprint)])
            .build()?;
        Ok(result)
    }

    fn sarif_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore: &Datastore = &self.datastore;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        let mut findings = Vec::with_capacity(group_metadata.len());
        for metadata in group_metadata {
            let matches = self.get_matches(&metadata)?;
            let match_group = MatchGroup::new(metadata, matches);
            findings.push(self.make_sarif_result(&match_group)?);
        }

        let run = sarif::RunBuilder::default()
            .tool(noseyparker_sarif_tool()?)
            .results(findings)
            .build()?;

        let sarif = sarif::SarifBuilder::default()
            .version(sarif::Version::V2_1_0.to_string())
            // .schema("https://docs.oasis-open.org/sarif/sarif/v2.1.0/cos02/schemas/sarif-schema-2.1.0.json")
            .schema(sarif::SCHEMA_URL)
            .runs([run])
            .build()?;

        serde_json::to_writer(&mut writer, &sarif)?;
        writeln!(writer)?;

        Ok(())
    }
}

/// Load the rules used during the scan for the runs.tool.driver.rules array property
fn noseyparker_sarif_rules() -> Result<Vec<sarif::ReportingDescriptor>> {
    // FIXME: this ignores any non-builtin rules
    get_builtin_rules()
        .context("Failed to load builtin rules")?
        .iter_rules()
        .map(|rule| {
            let help = sarif::MultiformatMessageStringBuilder::default()
                .text(&rule.references.join("\n"))
                .build()?;

            // FIXME: add better descriptions to Nosey Parker rules
            let description = sarif::MultiformatMessageStringBuilder::default()
                .text(&rule.pattern)
                .build()?;

            let rule = sarif::ReportingDescriptorBuilder::default()
                .id(&rule.name) // FIXME: nosey parker rules need to have stable, unique IDs, preferably without spaces
                // .name(&rule.name)  // FIXME: populate this once we have proper IDs
                .short_description(description)
                // .full_description(description)  // FIXME: populate this
                .help(help) // FIXME: provide better help messages for NP rules that we can include here
                // .help_uri() // FIXME: populate this
                .build()?;
            Ok(rule)
        })
        .collect::<Result<Vec<_>>>()
}

fn noseyparker_sarif_tool() -> Result<sarif::Tool> {
    sarif::ToolBuilder::default()
        .driver(
            sarif::ToolComponentBuilder::default()
                .name(env!("CARGO_PKG_NAME").to_string())
                .semantic_version(env!("CARGO_PKG_VERSION").to_string())
                .full_name(concat!("Nosey Parker ", env!("CARGO_PKG_VERSION"))) // FIXME: move into cargo.toml metadata, extract here; see https://docs.rs/cargo_metadata/latest/cargo_metadata/
                .organization("Praetorian, Inc") // FIXME: move into cargo.toml metadata, extract here
                .information_uri(env!("CARGO_PKG_HOMEPAGE").to_string())
                .download_uri(env!("CARGO_PKG_REPOSITORY").to_string())
                // .full_description() // FIXME: populate with some long description, like the text from the README.md
                .short_description(
                    sarif::MultiformatMessageStringBuilder::default()
                        .text(env!("CARGO_PKG_DESCRIPTION"))
                        .build()?,
                )
                .rules(noseyparker_sarif_rules()?)
                .build()?,
        )
        .build()
        .map_err(|e| e.into())
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

lazy_static! {
    static ref STYLE_FINDING_HEADING: console::Style =
        console::Style::new().bold().bright().white();
    static ref STYLE_RULE: console::Style = console::Style::new().bright().bold().blue();
    static ref STYLE_HEADING: console::Style = console::Style::new().bold();
    static ref STYLE_MATCH: console::Style = console::Style::new().yellow();
    static ref STYLE_METADATA: console::Style = console::Style::new().bright().blue();
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

impl Display for MatchGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", STYLE_RULE.apply_to(self.rule_name()),)?;

        // write out status if set
        if let Some(status) = self.metadata.status {
            let status = match status {
                Status::Accept => "Accept",
                Status::Reject => "Reject",
            };
            writeln!(f, "{} {}", STYLE_HEADING.apply_to("Status:"), status)?;
        };

        // write out comment if set
        if let Some(comment) = &self.metadata.comment {
            writeln!(f, "{} {}", STYLE_HEADING.apply_to("Comment:"), comment)?;
        };

        // write out the group on one line if it's single-line, and multiple lines otherwise
        let g = self.group_input();
        let match_heading = STYLE_HEADING.apply_to("Match:");
        if g.contains(&b'\n') {
            writeln!(f, "{match_heading}")?;
            writeln!(f)?;
            writeln!(indented(f).with_str("    "), "{}", STYLE_MATCH.apply_to(Escaped(g)))?;
            writeln!(f)?;
        } else {
            writeln!(f, "{} {}", match_heading, STYLE_MATCH.apply_to(Escaped(g)))?;
        }

        // write out count if not all matches are displayed
        if self.num_matches() != self.total_matches() {
            writeln!(
                f,
                "{}",
                STYLE_HEADING.apply_to(format!(
                    "Showing {}/{} occurrences:",
                    self.num_matches(),
                    self.total_matches()
                ))
            )?;
        }
        writeln!(f)?;

        // write out matches
        let mut f = indented(f).with_str("    ");
        for (i, ReportMatch { ps, md, m, .. }) in self.matches.iter().enumerate() {
            let i = i + 1;
            writeln!(
                f,
                "{}",
                STYLE_HEADING.apply_to(format!("Occurrence {i}/{}", self.total_matches())),
            )?;

            let blob_metadata = {
                format!(
                    "{} bytes, {}, {}",
                    md.num_bytes(),
                    md.mime_essence().unwrap_or("unknown type"),
                    md.charset().unwrap_or("unknown charset"),
                )
            };

            for p in ps.iter() {
                match p {
                    Provenance::File(e) => {
                        writeln!(
                            f,
                            "{} {}",
                            STYLE_HEADING.apply_to("File:"),
                            STYLE_METADATA.apply_to(e.path.display()),
                        )?;
                    }
                    Provenance::GitRepo(e) => {
                        writeln!(
                            f,
                            "{} {}",
                            STYLE_HEADING.apply_to("Git repo:"),
                            STYLE_METADATA.apply_to(e.repo_path.display()),
                        )?;
                        if let Some(cs) = &e.commit_provenance {
                            let cmd = &cs.commit_metadata;
                            let msg = BStr::new(cmd.message.lines().next().unwrap_or(&[]));
                            let atime = cmd
                                .author_timestamp
                                .format(time::macros::format_description!("[year]-[month]-[day]"));
                            writeln!(
                                f,
                                "{} {} in {}",
                                STYLE_HEADING.apply_to("Commit:"),
                                cs.commit_kind,
                                STYLE_METADATA.apply_to(cmd.commit_id),
                            )?;
                            writeln!(f)?;
                            writeln!(
                                indented(&mut f).with_str("    "),
                                "{}     {} <{}>\n\
                                 {}       {}\n\
                                 {}    {}\n\
                                 {}       {}",
                                STYLE_HEADING.apply_to("Author:"),
                                cmd.author_name,
                                cmd.author_email,
                                STYLE_HEADING.apply_to("Date:"),
                                atime,
                                STYLE_HEADING.apply_to("Summary:"),
                                msg,
                                STYLE_HEADING.apply_to("Path:"),
                                cs.blob_path,
                            )?;
                            writeln!(f)?;
                        }
                    }
                }
            }

            writeln!(
                f,
                "{} {} ({})",
                STYLE_HEADING.apply_to("Blob:"),
                STYLE_METADATA.apply_to(&m.blob_id),
                STYLE_METADATA.apply_to(blob_metadata),
            )?;

            writeln!(f, "{} {}", STYLE_HEADING.apply_to("Lines:"), &m.location.source_span,)?;
            writeln!(f)?;
            writeln!(
                indented(&mut f).with_str("    "),
                "{}{}{}",
                Escaped(&m.snippet.before),
                STYLE_MATCH.apply_to(Escaped(&m.snippet.matching)),
                Escaped(&m.snippet.after)
            )?;
            writeln!(f)?;
        }

        Ok(())
    }
}
