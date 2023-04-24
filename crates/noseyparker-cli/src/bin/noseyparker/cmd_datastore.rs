use anyhow::Result;
use tracing::info;

use crate::args;
use noseyparker::datastore::Datastore;

pub fn run(_global_args: &args::GlobalArgs, args: &args::DatastoreArgs) -> Result<()> {
    match &args.command {
        args::DatastoreCommand::Init(args) => cmd_datastore_init(args),
    }
}

fn cmd_datastore_init(args: &args::DatastoreInitArgs) -> Result<()> {
    let datastore = Datastore::create(&args.datastore)?;
    info!("Initialized new datastore at {}", &datastore.root_dir().display());
    Ok(())
}
