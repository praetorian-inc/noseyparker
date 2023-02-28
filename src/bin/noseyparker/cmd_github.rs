use anyhow::{bail, Result};

use crate::args::{GitHubArgs, GitHubReposListArgs, GlobalArgs, Reportable};
use noseyparker::github;

pub fn run(global_args: &GlobalArgs, args: &GitHubArgs) -> Result<()> {
    use crate::args::{GitHubCommand::*, GitHubReposCommand::*};
    match &args.command {
        Repos(List(args)) => list_repos(global_args, args),
    }
}

fn list_repos(_global_args: &GlobalArgs, args: &GitHubReposListArgs) -> Result<()> {
    if args.repo_specifiers.is_empty() {
        bail!("No repositories specified");
    }
    let repo_urls = github::enumerate_repo_urls(&github::RepoSpecifiers {
        user: args.repo_specifiers.user.clone(),
        organization: args.repo_specifiers.organization.clone(),
    })?;
    RepoReporter(repo_urls).report(&args.output_args)
}

struct RepoReporter(Vec<String>);

impl Reportable for RepoReporter {
    fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let repo_urls = &self.0;
        for repo_url in repo_urls {
            writeln!(writer, "{repo_url}")?;
        }
        Ok(())
    }

    fn json_format<W: std::io::Write>(&self, writer: W) -> Result<()> {
        let repo_urls = &self.0;
        serde_json::to_writer_pretty(writer, repo_urls)?;
        Ok(())
    }

    fn jsonl_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let repo_urls = &self.0;
        for repo_url in repo_urls {
            serde_json::to_writer(&mut writer, repo_url)?;
            writeln!(&mut writer)?;
        }
        Ok(())
    }

    fn sarif_format<W: std::io::Write>(&self, _writer: W) -> Result<()> {
        bail!("SARIF output not supported for this command")
    }
}
