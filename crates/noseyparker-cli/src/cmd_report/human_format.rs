use super::*;

impl DetailsReporter {
    pub fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let group_metadata = self.get_finding_metadata()?;
        let num_findings = group_metadata.len();
        for (finding_num, metadata) in group_metadata.into_iter().enumerate() {
            let finding_num = finding_num + 1;
            let matches = self.get_matches(&metadata)?;
            let finding = Finding { metadata, matches };
            writeln!(
                &mut writer,
                "{} (id {})",
                self.style_finding_heading(format!("Finding {finding_num}/{num_findings}")),
                self.style_id(&finding.metadata.finding_id),
            )?;
            writeln!(&mut writer, "{}", PrettyFinding(self, &finding))?;
        }
        Ok(())
    }
}

/// A wrapper type to allow human-oriented pretty-printing of a `Finding`.
pub struct PrettyFinding<'a>(&'a DetailsReporter, &'a Finding);

impl<'a> Display for PrettyFinding<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let PrettyFinding(reporter, finding) = self;
        writeln!(
            f,
            "{} {}",
            reporter.style_heading("Rule:"),
            reporter.style_rule(finding.rule_name())
        )?;

        // write out status if set: either `Accept`, `Reject`, or `Mixed` (when there are
        // conflicting match statuses within the finding)
        let statuses = &finding.metadata.statuses.0;
        let num_statuses = statuses.len();
        #[allow(clippy::comparison_chain)]
        if num_statuses > 1 {
            writeln!(f, "{} Mixed", reporter.style_heading("Status:"))?;
        } else if num_statuses == 1 {
            let status = match statuses[0] {
                Status::Accept => "Accept",
                Status::Reject => "Reject",
            };
            writeln!(f, "{} {status}", reporter.style_heading("Status:"))?;
        };

        // write out score if set
        if let Some(mean_score) = finding.metadata.mean_score {
            writeln!(f, "{} {mean_score:.3}", reporter.style_heading("Score:"))?;
        };

        // write out comment if set
        if let Some(comment) = &finding.metadata.comment {
            writeln!(f, "{} {comment}", reporter.style_heading("Comment:"))?;
        };

        let mut write_group =
            |group_heading: StyledObject<String>, g: &Group| -> std::fmt::Result {
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
                    "Showing {}/{} matches:",
                    finding.num_matches_available(),
                    finding.total_matches()
                ))
            )?;
        }
        writeln!(f)?;

        // write out matches
        let mut f = indented(f).with_str("    ");
        for (i, rm) in finding.matches.iter().enumerate() {
            let i = i + 1;
            let ReportMatch {
                provenance,
                blob_metadata,
                m,
                score,
                comment,
                status,
                redundant_to,
            } = rm;

            writeln!(
                f,
                "{} (id {})",
                reporter.style_heading(format!("Match {i}/{}", finding.total_matches())),
                reporter.style_id(&m.structural_id),
            )?;

            if !redundant_to.is_empty() {
                writeln!(
                    f,
                    "{} {}",
                    reporter.style_heading("Redundant to:"),
                    redundant_to.join(", "),
                )?;
            }

            // write out match status if set
            if let Some(status) = status {
                let status = match status {
                    Status::Accept => "Accept",
                    Status::Reject => "Reject",
                };
                writeln!(f, "{} {status}", reporter.style_heading("Status:"))?;
            }

            // write out match score if set
            if let Some(score) = score {
                writeln!(f, "{} {score:.3}", reporter.style_heading("Score:"))?;
            };

            // write out match comment if set
            if let Some(comment) = comment {
                writeln!(f, "{} {comment}", reporter.style_heading("Comment:"))?;
            };

            let blob_metadata = {
                format!(
                    "{} bytes, {}, {}",
                    blob_metadata.num_bytes(),
                    blob_metadata.mime_essence().unwrap_or("unknown type"),
                    blob_metadata.charset().unwrap_or("unknown charset"),
                )
            };

            for p in provenance.iter() {
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
                            let atime = cmd.author_timestamp.format(gix::date::time::format::SHORT);
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
                    // FIXME: implement this case properly
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
