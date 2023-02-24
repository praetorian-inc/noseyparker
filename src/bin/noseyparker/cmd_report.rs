use anyhow::{Context, Result};
use indenter::indented;
use lazy_static::lazy_static;
use noseyparker::rules::Rules;
use serde::{Deserialize, Serialize, Serializer};
use serde_sarif::sarif;
use std::fmt::{Display, Formatter, Write};

use noseyparker::bstring_escape::Escaped;
use noseyparker::datastore::{Datastore, MatchGroupMetadata};
use noseyparker::match_type::Match;
use noseyparker::provenance::Provenance;

use crate::args::{GlobalArgs, ReportArgs, Reportable};

pub fn run(_global_args: &GlobalArgs, args: &ReportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;
    DetailsReporter(datastore).report(&args.output_args)
}

struct DetailsReporter(Datastore);

impl Reportable for DetailsReporter {
    fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore = &self.0;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        let num_findings = group_metadata.len();
        for (finding_num, metadata) in group_metadata.into_iter().enumerate() {
            let finding_num = finding_num + 1;
            let matches = datastore
                .get_match_group_matches(&metadata, Some(3))
                .with_context(|| format!("Failed to get matches for group {metadata:?}"))?;
            let match_group = MatchGroup { metadata, matches };
            writeln!(
                &mut writer,
                "{} {}",
                STYLE_FINDING_HEADING
                    .apply_to(format!("Finding {finding_num}/{num_findings}:")),
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
                let matches = datastore
                    .get_match_group_matches(&metadata, None)
                    .with_context(|| format!("Failed to get matches for group {metadata:?}"))?;
                Ok(MatchGroup { metadata, matches })
            })
            .collect::<Result<Vec<MatchGroup>, anyhow::Error>>()?;
        let mut ser = serde_json::Serializer::pretty(writer);
        ser.collect_seq(es)?;
        Ok(())
    }

    fn jsonl_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore = &self.0;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;


        for metadata in group_metadata.into_iter() {
            let matches = datastore
                .get_match_group_matches(&metadata, None)
                .with_context(|| format!("Failed to get matches for group {metadata:?}"))?;
            let match_group = MatchGroup { metadata, matches };

            serde_json::to_writer(&mut writer, &match_group)
                .map_err(|e| e.into())
                .and_then(|()| writeln!(writer))?;
        }
        Ok(())
    }

    fn sarif_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore: &Datastore = &self.0;
        let group_metadata = datastore
            .get_match_group_metadata()
            .context("Failed to get match group metadata from datastore")?;

        // Will store every match for the runs.results array property
        let mut results: Vec<sarif::Result> = vec![];

        for metadata in group_metadata.into_iter() {
            let matches = datastore
                .get_match_group_matches(&metadata, None)
                .with_context(|| format!("Failed to get matches for group {metadata:?}"))?;

            // Will store every match location for the runs.results.location array property
            let mut locations: Vec<sarif::Location> = vec![];

            // Get the first blob id in the matches, will be written to the description in runs.results.message
            let first_matched_blob = match matches.first() {
                Some(m) => m.blob_id.to_string(),
                None => String::new(),
            };

            for m in matches.into_iter() {
                
                // Build the location for the match
                let location = sarif::LocationBuilder::default()
                    .physical_location(
                        sarif::PhysicalLocationBuilder::default()
                            .artifact_location(
                                sarif::ArtifactLocationBuilder::default()
                                    .uri(
                                        match &m.provenance {
                                            Provenance::File { path} => String::from(path.to_str().with_context(|| format!("Failed to convert path to string: {:?}", &m.provenance))?),
                                            Provenance::GitRepo { path } => String::from(path.to_str().with_context(|| format!("Failed to convert path to string: {:?}", &m.provenance))?),
                                        }
                                    )
                                    .build()?,
                            )
                            .region(
                                sarif::RegionBuilder::default()
                                    .start_line(m.location.source_span.start.line as i64)
                                    .start_column(m.location.source_span.start.column as i64)
                                    .end_line(m.location.source_span.end.line as i64)
                                    .end_column(m.location.source_span.end.column as i64)
                                    .snippet(
                                        sarif::ArtifactContentBuilder::default()
                                            .text(m.snippet.matching.to_string())
                                            .build()?,
                                    )
                                    .build()?,
                            )
                            .build()?,
                        )
                    .logical_locations(
                        vec![
                            sarif::LogicalLocationBuilder::default()
                                .kind("blob")
                                .name(m.blob_id.to_string())
                                .build()?
                        ]
                    )
                    .build()?;
                    
                locations.push(location);
            }

            // Build the result for the match
            let result = sarif::ResultBuilder::default()
                .rule_id(&metadata.rule_name)
                .message(
                    sarif::MessageBuilder::default()
                        .text(format!("Rule {} has detected a secret with {} matches.\nFirst blob id matched : {}", &metadata.rule_name, &metadata.num_matches, first_matched_blob))
                        .build()?,
                )
                .locations(locations)
                .level(sarif::ResultLevel::Warning.to_string())
                .build()?;
            
            results.push(result);
        }

        // Load the rules used during the scan for the runs.tool.driver.rules array property
        let rules = Rules::from_default_rules().with_context(|| format!("Failed to load default rules"))?;
        
        let sr = sarif::SarifBuilder::default()
            .version(sarif::Version::V2_1_0.to_string())
            .schema("https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0-rtm.4.json")
            .runs(
                vec![
                    sarif::RunBuilder::default()
                        .tool(
                            sarif::ToolBuilder::default()
                                .driver(
                                    sarif::ToolComponentBuilder::default()
                                        .name(env!("CARGO_PKG_NAME").to_string())
                                        .semantic_version(env!("CARGO_PKG_VERSION").to_string())
                                        .rules(
                                            rules.into_iter().map(|rule| {

                                                let help = match sarif::MultiformatMessageStringBuilder::default()
                                                    .text(&rule.references.join("\n"))
                                                    .build() {
                                                        Ok(help) => help,
                                                        Err(_) => sarif::MultiformatMessageStringBuilder::default().text("No help available".to_string()).build().unwrap()
                                                    };
                                                    

                                                // If we can't parse the regex pattern, we just no dot put it in the sarif report
                                                match sarif::MultiformatMessageStringBuilder::default()
                                                    .text(rule.pattern)
                                                    .build() {
                                                    Ok(description) => {
                                                        return sarif::ReportingDescriptorBuilder::default()
                                                            .id(&rule.name)
                                                            .name(&rule.name)
                                                            .short_description(description.clone())
                                                            .full_description(description.clone())
                                                            .help(help)
                                                            .build()
                                                    },
                                                    Err(_) => {
                                                        return sarif::ReportingDescriptorBuilder::default()
                                                            .id(&rule.name)
                                                            .name(&rule.name)
                                                            .build()
                                                    }
                                                };
                                            }).collect::<Result<Vec<_>, _>>()?
                                        )
                                        .build()?,
                                    )
                                    .build()?,
                        )
                        .results(results)
                        .build()?,
                ]
            )
            .build()?;

        serde_json::to_writer(&mut writer, &sr)
            .map_err(|e| e.into())
            .and_then(|()| writeln!(writer))?;

        Ok(())
    }
}

/// A group of matches that all have the same rule and capture group content
#[derive(Serialize, Deserialize)]
struct MatchGroup {
    #[serde(flatten)]
    metadata: MatchGroupMetadata,
    matches: Vec<Match>,
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

// XXX this implementation is grotty
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
        for (i, m) in self.matches.iter().enumerate() {
            let i = i + 1;
            writeln!(
                f,
                "{}",
                STYLE_HEADING.apply_to(format!("Occurrence {}/{}", i, self.total_matches()))
            )?;
            match &m.provenance {
                Provenance::File { path } => {
                    writeln!(
                        f,
                        "{} {}",
                        STYLE_HEADING.apply_to("File:"),
                        STYLE_METADATA.apply_to(path.display())
                    )?;
                }
                Provenance::GitRepo { path } => {
                    writeln!(
                        f,
                        "{} {}",
                        STYLE_HEADING.apply_to("Git repo:"),
                        STYLE_METADATA.apply_to(path.display())
                    )?;
                    writeln!(
                        f,
                        "{} {}",
                        STYLE_HEADING.apply_to("Blob:"),
                        STYLE_METADATA.apply_to(&m.blob_id)
                    )?;
                }
            }
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
