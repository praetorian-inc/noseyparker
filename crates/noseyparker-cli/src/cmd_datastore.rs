use anyhow::{Context, Result};
use tracing::info;

use crate::args::{DatastoreArgs, DatastoreExportArgs, DatastoreInitArgs, GlobalArgs};
use noseyparker::datastore::Datastore;

pub fn run(global_args: &GlobalArgs, args: &DatastoreArgs) -> Result<()> {
    use crate::args::DatastoreCommand::*;
    match &args.command {
        Init(args) => cmd_datastore_init(global_args, args),
        Export(args) => cmd_datastore_export(global_args, args),
    }
}

fn cmd_datastore_init(global_args: &GlobalArgs, args: &DatastoreInitArgs) -> Result<()> {
    let datastore = Datastore::create(&args.datastore, global_args.advanced.sqlite_cache_size)?;
    info!("Initialized new datastore at {}", &datastore.root_dir().display());
    Ok(())
}

fn cmd_datastore_export(global_args: &GlobalArgs, args: &DatastoreExportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;
    let output_path = &args.output;

    // XXX Move this code into datastore.rs?

    use crate::args::DatastoreExportOutputFormat::*;
    match args.format {
        Tgz => {
            use flate2::write::GzEncoder;
            use std::ffi::OsStr;
            use std::path::Path;
            use tempfile::NamedTempFile;

            let write_tar = |output_path: &Path| -> Result<()> {
                let prefix: &OsStr = output_path.file_name().unwrap_or("datastore.tgz".as_ref());

                let tmp_output = match output_path.parent() {
                    Some(p) => NamedTempFile::with_prefix_in(prefix, p),
                    None => NamedTempFile::with_prefix(prefix),
                }?;

                let enc = GzEncoder::new(tmp_output, Default::default());
                let mut tar = tar::Builder::new(enc);

                let root_dir = datastore.root_dir();
                tar.append_path_with_name(root_dir.join(".gitignore"), ".gitignore")?;
                tar.append_path_with_name(root_dir.join("datastore.db"), "datastore.db")?;
                tar.append_dir_all("blobs", datastore.blobs_dir())?;
                let tmp_output = tar.into_inner()?.finish()?;

                tmp_output.persist(output_path)?;

                Ok(())
            };

            write_tar(output_path).context("Failed to write tarfile")?;

            info!(
                "Exported datastore at {} to {}",
                &datastore.root_dir().display(),
                output_path.display()
            );
        }
    }

    Ok(())
}
