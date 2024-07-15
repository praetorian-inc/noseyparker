use anyhow::{bail, Context, Result};
use url::Url;

use crate::args::{
    validate_github_api_url, GitHubArgs, GitHubOutputFormat, GitHubReposListArgs, GlobalArgs,
};
use crate::reportable::Reportable;
use noseyparker::github;

pub fn run(global_args: &GlobalArgs, args: &GitHubArgs) -> Result<()> {
    use crate::args::{GitHubCommand::*, GitHubReposCommand::*};
    match &args.command {
        Repos(List(args_list)) => list_repos(global_args, args_list, args.github_api_url.clone()),
    }
}

fn list_repos(global_args: &GlobalArgs, args: &GitHubReposListArgs, api_url: Url) -> Result<()> {
    if args.repo_specifiers.is_empty() {
        bail!("No repositories specified");
    }
    validate_github_api_url(&api_url, args.repo_specifiers.all_organizations);
    let repo_urls = github::enumerate_repo_urls(
        &github::RepoSpecifiers {
            user: args.repo_specifiers.user.clone(),
            organization: args.repo_specifiers.organization.clone(),
            all_organizations: args.repo_specifiers.all_organizations,
            repo_filter: args.repo_specifiers.repo_type.into(),
        },
        api_url,
        global_args.ignore_certs,
        None,
    )
    .context("Failed to enumerate GitHub repositories")?;
    let output = args
        .output_args
        .get_writer()
        .context("Failed to get output writer")?;
    RepoReporter(repo_urls).report(args.output_args.format, output)
}

struct RepoReporter(Vec<String>);

impl Reportable for RepoReporter {
    type Format = GitHubOutputFormat;

    fn report<W: std::io::Write>(&self, format: Self::Format, mut writer: W) -> Result<()> {
        match format {
            GitHubOutputFormat::Human => {
                let repo_urls = &self.0;
                for repo_url in repo_urls {
                    writeln!(writer, "{repo_url}")?;
                }
                Ok(())
            }

            GitHubOutputFormat::Json => {
                let repo_urls = &self.0;
                serde_json::to_writer_pretty(writer, repo_urls)?;
                Ok(())
            }

            GitHubOutputFormat::Jsonl => {
                let repo_urls = &self.0;
                for repo_url in repo_urls {
                    serde_json::to_writer(&mut writer, repo_url)?;
                    writeln!(&mut writer)?;
                }
                Ok(())
            }
        }
    }
}
