use serde_sarif::sarif;

use super::*;

impl DetailsReporter {
    fn make_sarif_result(&self, finding: &Finding) -> Result<sarif::Result> {
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
                let ReportMatch {
                    provenance,
                    blob_metadata,
                    m,
                    ..
                } = m;
                provenance.iter().map(move |p| {
                    let source_span = &m.location.source_span;
                    // let offset_span = &m.location.offset_span;

                    let additional_properties =
                        vec![(String::from("blob_metadata"), serde_json::json!(blob_metadata))];

                    let mut artifact_location = sarif::ArtifactLocationBuilder::default();
                    if let Some(path) = p.blob_path() {
                        artifact_location.uri(path.to_string_lossy());
                    }
                    let artifact_location = artifact_location.build()?;

                    let additional_properties =
                        std::collections::BTreeMap::from_iter(additional_properties);
                    let properties = sarif::PropertyBagBuilder::default()
                        .additional_properties(additional_properties)
                        .build()?;

                    let location = sarif::LocationBuilder::default()
                        .physical_location(
                            sarif::PhysicalLocationBuilder::default()
                                .artifact_location(artifact_location)
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

        let fingerprint_name = "match_group_content/sha256/v1".to_string();
        let fingerprint = metadata.finding_id.clone();

        // Build the result for the match
        let result = sarif::ResultBuilder::default()
            .rule_id(&metadata.rule_name)
            // .occurrence_count(locations.len() as i64)  // FIXME: enable?
            .message(message)
            .kind(sarif::ResultKind::Review.to_string())
            .locations(locations)
            .level(sarif::ResultLevel::Warning.to_string())
            .partial_fingerprints([(fingerprint_name, fingerprint)])
            .build()?;
        Ok(result)
    }

    pub fn sarif_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore: &Datastore = &self.datastore;
        let group_metadata = datastore
            .get_finding_metadata()
            .context("Failed to get match group metadata from datastore")?;

        let mut findings = Vec::with_capacity(group_metadata.len());
        for metadata in group_metadata {
            let matches = self.get_matches(&metadata)?;
            let finding = Finding::new(metadata, matches);
            findings.push(self.make_sarif_result(&finding)?);
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
