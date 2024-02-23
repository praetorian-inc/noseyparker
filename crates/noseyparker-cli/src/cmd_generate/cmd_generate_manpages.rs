use crate::args::{CommandLineArgs, GlobalArgs, ManPagesArgs};
use anyhow::Result;
use clap::CommandFactory;
use clap_mangen::generate_to;
use tracing::info;

pub fn run(_global_args: &GlobalArgs, args: &ManPagesArgs) -> Result<()> {
    let cmd = CommandLineArgs::command();
    generate_to(cmd, &args.output)?;
    info!("Wrote manpages to {}", args.output.display());
    Ok(())
}
