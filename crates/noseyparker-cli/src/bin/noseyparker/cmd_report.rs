use anyhow::{bail, Context, Result};
use bstr::{BStr, ByteSlice};
use indenter::indented;
use lazy_static::lazy_static;
use noseyparker::rules::Rules;
use serde::Serialize;
use serde_sarif::sarif;
use std::fmt::{Display, Formatter, Write};

use noseyparker::blob_metadata::BlobMetadata;
use noseyparker::bstring_escape::Escaped;
use noseyparker::datastore::{Datastore, MatchGroupMetadata};
use noseyparker::digest::sha1_hexdigest;
use noseyparker::match_type::Match;
use noseyparker::provenance::Provenance;
use noseyparker::provenance_set::ProvenanceSet;

use crate::args::{GlobalArgs, ReportArgs, Reportable};

pub fn run(_global_args: &GlobalArgs, args: &ReportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;
    DetailsReporter(datastore).report(&args.output_args)
}

struct DetailsReporter(Datastore);

impl DetailsReporter {
    fn get_matches(
        &self,
        metadata: &MatchGroupMetadata,
        limit: Option<usize>,
    ) -> Result<Vec<ReportMatch>> {
        Ok(self
            .0
            .get_match_group_data(metadata, limit)
            .with_context(|| format!("Failed to get match data for group {metadata:?}"))?
            .into_iter()
            .map(|(p, md, m)| ReportMatch { ps: p, md, m })
            .collect())
    }
}

impl Reportable for DetailsReporter {
    fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore = &self.0;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        let num_findings = group_metadata.len();
        for (finding_num, metadata) in group_metadata.into_iter().enumerate() {
            let finding_num = finding_num + 1;
            let matches = self.get_matches(&metadata, Some(3))?;
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

    fn json_format<W: std::io::Write>(&self, writer: W) -> Result<()> {
        let datastore = &self.0;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        // XXX is there some nice way to do this serialization without first building a vec?
        let es = group_metadata
            .into_iter()
            .map(|metadata| {
                let matches = self.get_matches(&metadata, None)?;
                Ok(MatchGroup { metadata, matches })
            })
            .collect::<Result<Vec<MatchGroup>, anyhow::Error>>()?;
        serde_json::to_writer_pretty(writer, &es)?;
        Ok(())
    }

    fn jsonl_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore = &self.0;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        for metadata in group_metadata.into_iter() {
            let matches = self.get_matches(&metadata, None)?;
            let match_group = MatchGroup { metadata, matches };

            serde_json::to_writer(&mut writer, &match_group)?;
            writeln!(writer)?;
        }
        Ok(())
    }

    fn sarif_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore: &Datastore = &self.0;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        // Will store every match for the runs.results array property
        let results: Vec<sarif::Result> = group_metadata
            .into_iter()
            .map(|metadata| {
                let matches = self.get_matches(&metadata, None)?;

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
                    .into_iter()
                    .map(|ReportMatch { ps, md, m }| {
                        let source_span = &m.location.source_span;
                        // let offset_span = &m.location.offset_span;

                        // FIXME: rework for the expanded git provenance data
                        let uri = "FIXME: rework for the expanded git provenance data".to_string();

                        let properties = sarif::PropertyBagBuilder::default()
                            .additional_properties([
                                (String::from("mime_essence"), serde_json::json!(md.mime_essence)),
                                (String::from("charset"), serde_json::json!(md.charset)),
                                (String::from("num_bytes"), serde_json::json!(md.num_bytes)),
                            ])
                            .build()?;

                        let location = sarif::LocationBuilder::default()
                            .physical_location(
                                sarif::PhysicalLocationBuilder::default()
                                    .artifact_location(
                                        sarif::ArtifactLocationBuilder::default()
                                            .uri(uri)
                                            .build()?,
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
                    .partial_fingerprints([(
                        "match_group_content/sha256/v1".to_string(),
                        sha1_fingerprint,
                    )])
                    .build()?;
                Ok(result)
            })
            .collect::<Result<_>>()?;

        let run = sarif::RunBuilder::default()
            .tool(noseyparker_sarif_tool()?)
            // .artifacts([ ])  // FIXME: add an entry for each blob with findings here; for each scanned git repo, add "nested artifacts" for each blob
            .results(results)
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
    Rules::from_default_rules()
        .context("Failed to load default rules")?
        .into_iter()
        .map(|rule| {
            let help = sarif::MultiformatMessageStringBuilder::default()
                .text(&rule.references.join("\n"))
                .build()?;

            // FIXME: add better descriptions to Nosey Parker rules
            let description = sarif::MultiformatMessageStringBuilder::default()
                .text(rule.pattern)
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

/// A group of matches that all have the same rule and capture group content
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
        writeln!(f, "{}", STYLE_RULE.apply_to(self.rule_name()))?;

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

        // print matches
        let mut f = indented(f).with_str("    ");
        for (i, ReportMatch { ps, md, m }) in self.matches.iter().enumerate() {
            let i = i + 1;
            writeln!(
                f,
                "{}",
                STYLE_HEADING.apply_to(format!("Occurrence {}/{}", i, self.total_matches()))
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
                            let ctime = cmd.committer_timestamp.format(time::macros::format_description!("[year]-[month]-[day]"));
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
                                 {}  {} <{}>\n\
                                 {}       {}\n\
                                 {}    {}",

                                STYLE_HEADING.apply_to("Author:"),
                                cmd.author_name,
                                cmd.author_email,

                                STYLE_HEADING.apply_to("Committer:"),
                                cmd.committer_name,
                                cmd.committer_email,

                                STYLE_HEADING.apply_to("Date:"),
                                ctime,

                                STYLE_HEADING.apply_to("Summary:"),
                                msg,
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
