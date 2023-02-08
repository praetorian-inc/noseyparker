use anyhow::{bail, Context, Result};
use tracing::{debug, warn};

use crate::args::{GlobalArgs, GitHubArgs, GitHubReposListArgs, Reportable};
use noseyparker::github;

pub fn run(global_args: &GlobalArgs, args: &GitHubArgs) -> Result<()> {
    use crate::args::{GitHubCommand::*, GitHubReposCommand::*};
    match &args.command {
        Repos(List(args)) => list_repos(global_args, args),
    }
}

/// The name of the environment variable to look for a personal access token in.
///
/// NOTE: this variable needs to match the top-level help documentation in args.rs
const GITHUB_TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";

fn list_repos(_global_args: &GlobalArgs, args: &GitHubReposListArgs) -> Result<()> {
    if args.repo_specifiers.is_empty() {
        bail!("No repositories specified");
    }

    let client = {
        let mut builder = github::ClientBuilder::new();
        match std::env::var(GITHUB_TOKEN_ENV_VAR) {
            Err(std::env::VarError::NotPresent) => {
                debug!("No GitHub access token provided; using unauthenticated API access.");
            }
            Err(std::env::VarError::NotUnicode(_s)) => {
                bail!("Value of {} environment variable is ill-formed", GITHUB_TOKEN_ENV_VAR);
            }
            Ok(val) => {
                debug!("Using GitHub personal access token from {GITHUB_TOKEN_ENV_VAR} environment variable");
                builder = builder
                    .auth(github::Auth::PersonalAccessToken(secrecy::SecretString::from(val)));
            }
        }
        builder
            .build()
            .context("Failed to initialize GitHub client")?
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to initialize async runtime")?;

    let result = runtime.block_on(async {
        let mut repo_urls: Vec<String> = Vec::new();

        // Get rate limit first thing.
        // If there are connectivity issues, this is likely to reveal them quickly.
        //
        // This also makes it a little bit simpler to test this code, as the very first request
        // made by `github repos list` will always be the same, regardless of which repo specifiers
        // are given.
        let rate_limit = client.get_rate_limit().await?;
        debug!("GitHub rate limits: {:?}", rate_limit.rate);

        for username in &args.repo_specifiers.user {
            let mut repo_page = Some(client.get_user_repos(username).await?);
            while let Some(page) = repo_page {
                repo_urls.extend(page.items.iter().map(|r| &r.clone_url).cloned());
                repo_page = client.next_page(page).await?;
            }
        }

        for orgname in &args.repo_specifiers.organization {
            let mut repo_page = Some(client.get_org_repos(orgname).await?);
            while let Some(page) = repo_page {
                repo_urls.extend(page.items.iter().map(|r| &r.clone_url).cloned());
                repo_page = client.next_page(page).await?;
            }
        }

        repo_urls.sort();
        repo_urls.dedup();

        Ok::<Vec<String>, noseyparker::github::Error>(repo_urls)
    });

    match result {
        Ok(repo_urls) => {
            RepoReporter(repo_urls).report(&args.output_args)?;
        }
        Err(noseyparker::github::Error::RateLimited { wait, .. }) => {
            warn!("Rate limit exceeded: Would need to wait for {:?} before retrying", wait);
            result?;
        }
        Err(err) => bail!(err),
    }

    Ok(())
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
}
