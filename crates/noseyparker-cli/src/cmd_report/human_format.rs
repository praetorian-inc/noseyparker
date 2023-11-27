use super::*;

impl DetailsReporter {
    pub fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
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
                self.style_finding_heading(format!("Finding {finding_num}/{num_findings}:")),
                PrettyMatchGroup(self, &match_group),
            )?;
        }
        Ok(())
    }
}


/// A wrapper type to allow human-oriented pretty-printing of a `MatchGroup`.
pub struct PrettyMatchGroup<'a>(&'a DetailsReporter, &'a MatchGroup);

impl <'a> Display for PrettyMatchGroup<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let PrettyMatchGroup(reporter, group) = self;
        writeln!(f, "{}", reporter.style_rule(group.rule_name()))?;

        // write out status if set
        if let Some(status) = group.metadata.status {
            let status = match status {
                Status::Accept => "Accept",
                Status::Reject => "Reject",
            };
            writeln!(f, "{} {}", reporter.style_heading("Status:"), status)?;
        };

        // write out comment if set
        if let Some(comment) = &group.metadata.comment {
            writeln!(f, "{} {}", reporter.style_heading("Comment:"), comment)?;
        };

        // write out the group on one line if it's single-line, and multiple lines otherwise
        let g = group.group_input();
        let match_heading = reporter.style_heading("Match:");
        if g.contains(&b'\n') {
            writeln!(f, "{match_heading}")?;
            writeln!(f)?;
            writeln!(indented(f).with_str("    "), "{}", reporter.style_match(Escaped(g)))?;
            writeln!(f)?;
        } else {
            writeln!(f, "{} {}", match_heading, reporter.style_match(Escaped(g)))?;
        }

        // write out count if not all matches are displayed
        if group.num_matches() != group.total_matches() {
            writeln!(
                f,
                "{}",
                reporter.style_heading(format!(
                    "Showing {}/{} occurrences:",
                    group.num_matches(),
                    group.total_matches()
                ))
            )?;
        }
        writeln!(f)?;

        // write out matches
        let mut f = indented(f).with_str("    ");
        for (i, ReportMatch { ps, md, m, .. }) in group.matches.iter().enumerate() {
            let i = i + 1;
            writeln!(
                f,
                "{}",
                reporter.style_heading(format!("Occurrence {i}/{}", group.total_matches())),
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
                        if let Some(cs) = &e.commit_provenance {
                            let cmd = &cs.commit_metadata;
                            let msg = BStr::new(cmd.message.lines().next().unwrap_or(&[]));
                            let atime = cmd
                                .author_timestamp
                                .format(time::macros::format_description!("[year]-[month]-[day]"));
                            writeln!(
                                f,
                                "{} {} in {}",
                                reporter.style_heading("Commit:"),
                                cs.commit_kind,
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
