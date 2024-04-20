use anyhow::{Context, Result};
// use tracing::info;

use crate::args::{
    get_writer_for_file_or_stdout, AnnotationsArgs, AnnotationsExportArgs, AnnotationsImportArgs,
    GlobalArgs,
};

use noseyparker::datastore::Datastore;

pub fn run(global_args: &GlobalArgs, args: &AnnotationsArgs) -> Result<()> {
    use crate::args::AnnotationsCommand::*;
    match &args.command {
        Import(args) => cmd_annotations_import(global_args, args),
        Export(args) => cmd_annotations_export(global_args, args),
    }
}

fn cmd_annotations_import(global_args: &GlobalArgs, args: &AnnotationsImportArgs) -> Result<()> {
    let _datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;

    todo!();
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

// Annotation matching.
// ====================
//
// How do you know when an annotation applies to a match?
//
// Given: an annotation A and a match M
// Output: 1 if A applies to M, and 0 otherwise
//
// There are many possible ways to conclude that the annotation A applies to match M, some more
// certain than others:
//
// - Exact match: A.match_id = M.id
// - Nearly exact match, but accounting for changing rule: Equal values of (rule_text_id, groups, blob_id, start_byte, end_byte)
// - Finding-based match:
//   a. Equal values of (rule_structural_id, groups, snippet)
//   b. Equal values of (rule_text_id, groups, snippet)
//   c. Equal values of (rule name, groups, snippet)
// - Fuzzy matching:
//   - Equal values of (rule_text_id, groups, blob_id) and overlapping (start_byte, end_byte)
//   - Equal values of (blob_id, groups, start_byte, end_byte)
//   - Equal values of (blob_id, groups) and overlapping (start_byte, end_byte)
