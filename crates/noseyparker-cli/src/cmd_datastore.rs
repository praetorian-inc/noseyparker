use anyhow::Result;
use tracing::info;

use crate::args;
use noseyparker::datastore::Datastore;

pub fn run(global_args: &args::GlobalArgs, args: &args::DatastoreArgs) -> Result<()> {
    match &args.command {
        args::DatastoreCommand::Init(args) => cmd_datastore_init(global_args, args),
    }
}

fn cmd_datastore_init(
    global_args: &args::GlobalArgs,
    args: &args::DatastoreInitArgs,
) -> Result<()> {
    let datastore = Datastore::create(&args.datastore, global_args.advanced.sqlite_cache_size)?;
    info!("Initialized new datastore at {}", &datastore.root_dir().display());
    Ok(())
}
