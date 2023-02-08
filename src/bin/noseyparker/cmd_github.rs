use anyhow::{Context, Result, bail};
use tracing::{debug, warn};

use crate::args;
use noseyparker::github;

pub fn run(global_args: &args::GlobalArgs, args: &args::GitHubArgs) -> Result<()> {
    use args::GitHubCommand::*;
    use args::GitHubReposCommand::*;
    match &args.command {
        Repos(List(args)) => list_repos(global_args, args)
    }
}

const GITHUB_TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";

fn list_repos(_global_args: &args::GlobalArgs, args: &args::GitHubReposListArgs) -> Result<()> {
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
                debug!("Using GitHub personal access token from {} environment variable", GITHUB_TOKEN_ENV_VAR);
                builder = builder.auth(github::Auth::PersonalAccessToken(secrecy::SecretString::from(val)));
            }
        }
        builder.build().context("Failed to initialize GitHub client")?
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to initialize async runtime")?;

    for username in &args.repo_specifiers.user {
        let result = runtime.block_on(async {
            let mut repo_page = Some(client.get_user_repos(username).await?);
            while let Some(page) = repo_page {
                for repo in page.items.iter() {
                    // println!("{:#?}", repo);
                    println!("{:?}", repo.clone_url);
                }
                repo_page = client.next_page(page).await?;
            }
            println!("{:#?}", client.get_rate_limit().await?);
            Ok::<(), noseyparker::github::Error>(())
        });
        match result {
            Ok(()) => {}
            Err(noseyparker::github::Error::RateLimited { wait, .. }) => {
                warn!("Rate limit exceeded: Would need to wait for {:?} before retrying", wait);
                result?;
            }
            Err(err) => bail!(err),
        }
    }

    return Ok(());

    /*
    let mut writer = args
        .output_args
        .get_writer()
        .context("Failed to open output destination for writing")?;

    let run_inner = move || -> std::io::Result<()> {
        match &args.output_args.format {
            args::OutputFormat::Human => {
                // writeln!(writer)?;
                // let table = summary_table(summary);
                // // FIXME: this doesn't preserve ANSI styling on the table
                // table.print(&mut writer)?;
            }
            args::OutputFormat::Json => {
                // serde_json::to_writer_pretty(&mut writer, &summary)?;
            }
            args::OutputFormat::Jsonl => {
                // for entry in summary.0.iter() {
                //     serde_json::to_writer(&mut writer, entry)?;
                //     writeln!(&mut writer)?;
                // }
            }
        }
        Ok(())
    };
    match run_inner() {
        // Ignore SIGPIPE errors, like those that can come from piping to `head`
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => { Ok(()) }
        Err(e) => Err(e)?,
        Ok(()) => Ok(()),
    }
    */
}

/*
pub fn summary_table(summary: MatchSummary) -> prettytable::Table {
    use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
    use prettytable::row;

    let f = FormatBuilder::new()
        // .column_separator('│')
        // .separators(&[LinePosition::Title], LineSeparator::new('─', '┼', '├', '┤'))
        .column_separator(' ')
        .separators(&[LinePosition::Title], LineSeparator::new('─', '─', '─', '─'))
        .padding(1, 1)
        .build();

    let mut table: prettytable::Table = summary
        .0
        .into_iter()
        .map(|e| row![
             l -> &e.rule_name,
             r -> HumanCount(e.distinct_count.try_into().unwrap()),
             r -> HumanCount(e.total_count.try_into().unwrap())
        ])
        .collect();
    table.set_format(f);
    table.set_titles(row![lb -> "Rule", cb -> "Distinct Matches", cb -> "Total Matches"]);
    table
}
*/
