use anyhow::{Context, Result, bail};
// use chrono::{DateTime, Utc};
// use reqwest::Url;
// use serde::Deserialize;
// use std::collections::BTreeMap;

use crate::args;
use noseyparker::github;

pub fn run(global_args: &args::GlobalArgs, args: &args::GitHubArgs) -> Result<()> {
    use args::GitHubCommand::*;
    use args::GitHubReposCommand::*;
    match &args.command {
        Repos(List(args)) => list_repos(global_args, args)
    }
}

fn list_repos(_global_args: &args::GlobalArgs, args: &args::GitHubReposListArgs) -> Result<()> {
    if args.repo_specifiers.is_empty() {
        bail!("No repositories specified");
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to initialize async runtime")?;

    let client = github::Client::new()
        .context("Failed to initialize GitHub client")?;

    let rate_limit = runtime.block_on(client.rate_limit())?;
    println!("{:#?}", rate_limit);

    for username in &args.repo_specifiers.user {
        let user = runtime.block_on(client.user(username))?;
        println!("{:#?}", user);
        let repos = runtime.block_on(client.user_repos(username))?;
        for repo in repos.iter() {
            println!("{:#?}", repo);
        }
    }

    let rate_limit = runtime.block_on(client.rate_limit())?;
    println!("{:#?}", rate_limit);

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
