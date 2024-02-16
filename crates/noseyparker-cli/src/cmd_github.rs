use anyhow::{bail, Context, Result};
use clap::error::ErrorKind;
use clap::CommandFactory;
use url::Url;

use crate::args::{
    CommandLineArgs, GitHubArgs, GitHubOutputFormat, GitHubReposListArgs, GlobalArgs,
};
use crate::reportable::Reportable;
use noseyparker::github;

pub fn run(global_args: &GlobalArgs, args: &GitHubArgs) -> Result<()> {
    use crate::args::{GitHubCommand::*, GitHubReposCommand::*};
    match &args.command {
        Repos(List(args_list)) => list_repos(global_args, args_list, args.github_api_url.clone()),
    }
}

fn list_repos(_global_args: &GlobalArgs, args: &GitHubReposListArgs, api_url: Url) -> Result<()> {
    if args.repo_specifiers.is_empty() && !args.repo_specifiers.all_organizations {
        bail!("No repositories specified");
    }
    if let Some(host) = api_url.host_str() {
        if host == "api.github.com" && args.repo_specifiers.all_organizations {
            let mut cmd = CommandLineArgs::command();
            let err = cmd.error(
                ErrorKind::MissingRequiredArgument,
                "The custom GitHub API URL was not specified",
            );
            err.exit();
        }
    }
    let repo_urls = github::enumerate_repo_urls(
        &github::RepoSpecifiers {
            user: args.repo_specifiers.user.clone(),
            organization: args.repo_specifiers.organization.clone(),
            all_organizations: args.repo_specifiers.all_organizations,
        },
        api_url,
        args.ignore_certs,
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
