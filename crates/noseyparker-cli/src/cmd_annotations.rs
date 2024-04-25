use anyhow::{Context, Result};
// use tracing::info;
use tracing::debug;

use crate::args::{AnnotationsArgs, AnnotationsExportArgs, AnnotationsImportArgs, GlobalArgs};
use crate::util::{get_reader_for_file_or_stdin, get_writer_for_file_or_stdout};

use noseyparker::datastore::Annotations;
use noseyparker::datastore::Datastore;

pub fn run(global_args: &GlobalArgs, args: &AnnotationsArgs) -> Result<()> {
    use crate::args::AnnotationsCommand::*;
    match &args.command {
        Import(args) => cmd_annotations_import(global_args, args),
        Export(args) => cmd_annotations_export(global_args, args),
    }
}

fn cmd_annotations_import(global_args: &GlobalArgs, args: &AnnotationsImportArgs) -> Result<()> {
    let mut datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;

    let input = get_reader_for_file_or_stdin(args.input.as_ref())?;

    let annotations: Annotations =
        serde_json::from_reader(input).context("Failed to read JSON input")?;
    debug!(
        "Read {} match and {} finding annotations",
        annotations.match_annotations.len(),
        annotations.finding_annotations.len()
    );
    datastore.import_annotations(&annotations)?;

    Ok(())
}

fn cmd_annotations_export(global_args: &GlobalArgs, args: &AnnotationsExportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;

    let output = get_writer_for_file_or_stdout(args.output.as_ref())
        .context("Failed to open output for writing")?;

    let annotations = datastore
        .get_annotations()
        .context("Failed to get annotations")?;

    serde_json::to_writer(output, &annotations).context("Failed to write JSON output")?;

    Ok(())
}
