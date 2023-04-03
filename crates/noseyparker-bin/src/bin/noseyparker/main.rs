use anyhow::{Context, Result};
use tracing::debug;

mod args;
mod cmd_datastore;
mod cmd_github;
mod cmd_report;
mod cmd_rules;
mod cmd_scan;
mod cmd_summarize;

fn configure_tracing(global_args: &args::GlobalArgs) -> Result<()> {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_log::{AsLog, LogTracer};

    let filter = match global_args.verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    LogTracer::builder()
        .with_max_level(filter.as_log())
        .init()?;

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
      .with_max_level(filter)
      .with_ansi(global_args.use_color())
      .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

fn configure_rlimits() -> Result<()> {
    use rlimit::Resource;
    use std::cmp::max;

    const NOFILE_LIMIT: u64 = 16384;
    let (soft, hard) = Resource::NOFILE.get()?;
    let soft = max(soft, NOFILE_LIMIT);
    let hard = max(hard, NOFILE_LIMIT);
    Resource::NOFILE.set(soft, hard)?;
    debug!("Set {} limit to ({}, {})", Resource::NOFILE.as_name(), soft, hard);
    Ok(())
}

fn try_main() -> Result<()> {
    // Print a stack trace in case of panic.
    // This should have no overhead in normal execution.
    std::env::set_var("RUST_BACKTRACE", "1");

    let args = &args::CommandLineArgs::parse_args();
    let global_args = &args.global_args;

    let use_color = global_args.use_color();
    console::set_colors_enabled(use_color);
    console::set_colors_enabled_stderr(use_color);

    configure_tracing(&args.global_args).context("Failed to initialize logging")?;
    configure_rlimits().context("Failed to initialize resource limits")?;

    match &args.command {
        args::Command::Datastore(args) => cmd_datastore::run(global_args, args),
        args::Command::GitHub(args) => cmd_github::run(global_args, args),
        args::Command::Rules(args) => cmd_rules::run(global_args, args),
        args::Command::Scan(args) => cmd_scan::run(global_args, args),
        args::Command::Summarize(args) => cmd_summarize::run(global_args, args),
        args::Command::Report(args) => cmd_report::run(global_args, args),
    }
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("Error: {e:?}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    #[test]
    #[should_panic]
    fn failure() {
        assert_eq!(5, 42);
    }
}
