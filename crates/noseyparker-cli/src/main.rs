use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use anyhow::{Context, Result};
use tracing::debug;

mod args;
mod cmd_datastore;
mod cmd_generate;
mod cmd_github;
mod cmd_report;
mod cmd_rules;
mod cmd_scan;
mod cmd_summarize;
mod reportable;
mod rule_loader;
mod util;

use args::{CommandLineArgs, GlobalArgs};

/// Set up the logging / tracing system for the application.
fn configure_tracing(global_args: &GlobalArgs) -> Result<()> {
    use tracing_log::{AsLog, LogTracer};
    use tracing_subscriber::{filter::LevelFilter, EnvFilter};

    // Set the tracing level according to the `-q`/`--quiet` and `-v`/`--verbose` options
    let level_filter = if global_args.quiet {
        LevelFilter::ERROR
    } else {
        match global_args.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    };

    // Configure the bridge from the `log` crate to the `tracing` crate
    LogTracer::builder()
        .with_max_level(level_filter.as_log())
        .init()?;

    // Configure logging filters according to the `NP_LOG` environment variable
    let env_filter = EnvFilter::builder()
        .with_default_directive(level_filter.into())
        .with_env_var("NP_LOG")
        .from_env()
        .context("Failed to parse filters from NP_LOG environment variable")?;

    // Install the global tracing subscriber
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(global_args.use_color(std::io::stderr()))
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

/// Set the process rlimits according to the global arguments.
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

/// Enable or disable colored output according to the global arguments.
fn configure_color(global_args: &GlobalArgs) {
    console::set_colors_enabled(global_args.use_color(std::io::stdout()));
    console::set_colors_enabled_stderr(global_args.use_color(std::io::stderr()));
}

/// Enable or disable backtraces for the process according to the global arguments.
fn configure_backtraces(global_args: &GlobalArgs) {
    if global_args.advanced.enable_backtraces {
        // Print a stack trace in case of panic.
        // This should have no overhead in normal execution.
        std::env::set_var("RUST_BACKTRACE", "1");
    }
}

fn try_main(args: &CommandLineArgs) -> Result<()> {
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
        args::Command::Generate(args) => cmd_generate::run(global_args, args),
    }
}

fn main() {
    let args = &CommandLineArgs::parse_args();
    if let Err(e) = try_main(args) {
        // Use the more verbose format that includes a backtrace when running with -vv or higher,
        // otherwise use a more compact one-line error format.
        if args.global_args.verbose > 1 {
            eprintln!("Error: {e:?}");
        } else {
            eprintln!("Error: {e:#}");
        }
        std::process::exit(2);
    }
}
