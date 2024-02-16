use super::*;

impl DetailsReporter {
    pub fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let datastore = &self.datastore;
        let group_metadata = datastore
            .get_finding_metadata()
            .context("Failed to get match group metadata from datastore")?;

        let num_findings = group_metadata.len();
        for (finding_num, metadata) in group_metadata.into_iter().enumerate() {
            let finding_num = finding_num + 1;
            let matches = self.get_matches(&metadata)?;
            let finding = Finding { metadata, matches };
            writeln!(&mut writer, "{}", self.style_finding_heading(format!("Finding {finding_num}/{num_findings}")))?;
            writeln!(&mut writer, "{}", PrettyFinding(self, &finding))?;
        }
        Ok(())
    }
}


/// A wrapper type to allow human-oriented pretty-printing of a `Finding`.
pub struct PrettyFinding<'a>(&'a DetailsReporter, &'a Finding);

impl <'a> Display for PrettyFinding<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let PrettyFinding(reporter, finding) = self;
        writeln!(f, "{} {}", reporter.style_heading("Rule:"), reporter.style_rule(finding.rule_name()))?;

        // write out status if set
        let statuses = &finding.metadata.statuses.0;
        if statuses.len() > 1 {
            writeln!(f, "{} {}", reporter.style_heading("Status:"), "Mixed")?;
        } else if statuses.len() == 1 {
            let status = match statuses[0] {
                Status::Accept => "Accept",
                Status::Reject => "Reject",
            };
            writeln!(f, "{} {}", reporter.style_heading("Status:"), status)?;
        };

        // write out comment if set
        if let Some(comment) = &finding.metadata.comment {
            writeln!(f, "{} {}", reporter.style_heading("Comment:"), comment)?;
        };

        let mut write_group = |group_heading: StyledObject<String>, g: &Group| {
            let g = &g.0;
            // write out the group on one line if it's single-line, and multiple lines otherwise
            if g.contains(&b'\n') {
                writeln!(f, "{group_heading}")?;
                writeln!(f)?;
                writeln!(indented(f).with_str("    "), "{}", reporter.style_match(Escaped(g)))?;
                writeln!(f)?;
            } else {
                writeln!(f, "{} {}", group_heading, reporter.style_match(Escaped(g)))?;
            }
            Ok(())
        };

        let gs = &finding.groups().0;
        if gs.len() > 1 {
            for (i, g) in gs.iter().enumerate() {
                let i = i + 1;
                let group_heading = reporter.style_heading(format!("Group {i}:"));
                write_group(group_heading, g)?;
            }
        } else {
            let group_heading = reporter.style_heading("Group:".into());
            write_group(group_heading, &gs[0])?;
        }

        // write out count if not all matches are displayed
        if finding.num_matches_available() != finding.total_matches() {
            writeln!(
                f,
                "{}",
                reporter.style_heading(format!(
                    "Showing {}/{} occurrences:",
                    finding.num_matches_available(),
                    finding.total_matches()
                ))
            )?;
        }
        writeln!(f)?;

        // write out matches
        let mut f = indented(f).with_str("    ");
        for (i, ReportMatch { ps, md, m, .. }) in finding.matches.iter().enumerate() {
            let i = i + 1;
            writeln!(
                f,
                "{}",
                reporter.style_heading(format!("Occurrence {i}/{}", finding.total_matches())),
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
                            reporter.style_heading("File:"),
                            reporter.style_metadata(e.path.display()),
                        )?;
                    }
                    Provenance::GitRepo(e) => {
                        writeln!(
                            f,
                            "{} {}",
                            reporter.style_heading("Git repo:"),
                            reporter.style_metadata(e.repo_path.display()),
                        )?;
                        if let Some(cs) = &e.first_commit {
                            let cmd = &cs.commit_metadata;
                            let msg = BStr::new(cmd.message.lines().next().unwrap_or(&[]));
                            let atime = cmd
                                .author_timestamp
                                .format(time::macros::format_description!("[year]-[month]-[day]"));
                            writeln!(
                                f,
                                "{} first seen in {}",
                                reporter.style_heading("Commit:"),
                                reporter.style_metadata(cmd.commit_id),
                            )?;
                            writeln!(f)?;
                            writeln!(
                                indented(&mut f).with_str("    "),
                                "{}     {} <{}>\n\
                                 {}       {}\n\
                                 {}    {}\n\
                                 {}       {}",
                                reporter.style_heading("Author:"),
                                cmd.author_name,
                                cmd.author_email,
                                reporter.style_heading("Date:"),
                                atime,
                                reporter.style_heading("Summary:"),
                                msg,
                                reporter.style_heading("Path:"),
                                cs.blob_path,
                            )?;
                            writeln!(f)?;
                        }
                    }
                    // TODO(overhaul): implement this case properly
                    Provenance::Extended(e) => {
                        writeln!(
                            f,
                            "{} {}",
                            reporter.style_heading("Extended Provenance:"),
                            reporter.style_metadata(e),
                        )?;
                    }
                }
            }

            writeln!(
                f,
                "{} {} ({})",
                reporter.style_heading("Blob:"),
                reporter.style_metadata(&m.blob_id),
                reporter.style_metadata(blob_metadata),
            )?;

            writeln!(f, "{} {}", reporter.style_heading("Lines:"), &m.location.source_span,)?;
            writeln!(f)?;
            writeln!(
                indented(&mut f).with_str("    "),
                "{}{}{}",
                Escaped(&m.snippet.before),
                reporter.style_match(Escaped(&m.snippet.matching)),
                Escaped(&m.snippet.after)
            )?;
            writeln!(f)?;
        }

        Ok(())
    }
}
