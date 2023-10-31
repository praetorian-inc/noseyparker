use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use anyhow::{Context, Result};
use tracing::debug;

mod args;
mod cmd_datastore;
mod cmd_github;
mod cmd_report;
mod cmd_rules;
mod cmd_scan;
mod cmd_shell_completions;
mod cmd_summarize;
mod rule_loader;
mod reportable;
mod util;

use args::GlobalArgs;

fn configure_tracing(global_args: &GlobalArgs) -> Result<()> {
    use tracing_log::{AsLog, LogTracer};
    use tracing_subscriber::{filter::LevelFilter, EnvFilter};

    let level_filter = match global_args.verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    LogTracer::builder()
        .with_max_level(level_filter.as_log())
        .init()?;

    let env_filter = EnvFilter::builder()
        .with_default_directive(level_filter.into())
        .with_env_var("NP_LOG")
        .from_env()
        .context("Failed to parse filters from NP_LOG environment variable")?;

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(global_args.use_color())
        .with_env_filter(env_filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

fn configure_rlimits(global_args: &GlobalArgs) -> Result<()> {
    use rlimit::Resource;
    use std::cmp::max;

    let nofile_limit = global_args.advanced.rlimit_nofile;
    let (soft, hard) = Resource::NOFILE.get()?;
    let soft = max(soft, nofile_limit);
    let hard = max(hard, nofile_limit);
    Resource::NOFILE.set(soft, hard)?;
    debug!("Set {} limit to ({}, {})", Resource::NOFILE.as_name(), soft, hard);
    Ok(())
}

fn configure_color(global_args: &GlobalArgs) {
    let use_color = global_args.use_color();
    console::set_colors_enabled(use_color);
    console::set_colors_enabled_stderr(use_color);
}

fn configure_backtraces(global_args: &GlobalArgs) {
    if global_args.advanced.enable_backtraces {
        // Print a stack trace in case of panic.
        // This should have no overhead in normal execution.
        std::env::set_var("RUST_BACKTRACE", "1");
    }
}

fn try_main() -> Result<()> {
    let args = &args::CommandLineArgs::parse_args();
    let global_args = &args.global_args;

    configure_backtraces(global_args);
    configure_color(global_args);
    configure_tracing(global_args).context("Failed to initialize logging")?;
    configure_rlimits(global_args).context("Failed to initialize resource limits")?;

    match &args.command {
        args::Command::Datastore(args) => cmd_datastore::run(global_args, args),
        args::Command::GitHub(args) => cmd_github::run(global_args, args),
        args::Command::Rules(args) => cmd_rules::run(global_args, args),
        args::Command::Scan(args) => cmd_scan::run(global_args, args),
        args::Command::Summarize(args) => cmd_summarize::run(global_args, args),
        args::Command::Report(args) => cmd_report::run(global_args, args),
        args::Command::ShellCompletions(args) => cmd_shell_completions::run(global_args, args),
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
