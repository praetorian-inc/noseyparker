use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::args::{AnnotationsArgs, AnnotationsExportArgs, AnnotationsImportArgs, GlobalArgs};

use noseyparker::blob_id::BlobId;
use noseyparker::datastore::{Datastore, Status};
use noseyparker::match_type::Groups;

pub fn run(global_args: &GlobalArgs, args: &AnnotationsArgs) -> Result<()> {
    use crate::args::AnnotationsCommand::*;
    match &args.command {
        Import(args) => cmd_annotations_import(global_args, args),
        Export(args) => cmd_annotations_export(global_args, args),
    }
}

fn cmd_annotations_import(global_args: &GlobalArgs, args: &AnnotationsImportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;

    todo!();
}

fn cmd_annotations_export(global_args: &GlobalArgs, args: &AnnotationsExportArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;

    todo!();
}

// -------------------------------------------------------------------------------------------------
// Annotation
// -------------------------------------------------------------------------------------------------
/// Represents an user-assigned annotation: a status and/or a comment
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Annotation {
    /// The content-based finding identifier for this group of matches
    pub finding_id: String,

    /// The name of the rule that detected each match
    pub rule_name: String,

    /// The textual identifier of the rule that detected each match
    pub rule_text_id: String,

    /// The structural identifier of the rule that detected the match
    pub rule_structural_id: String,

    /// The structural identifier of the match the annotations are associated with
    pub match_id: String,

    /// The blob where the match occurs
    pub blob_id: BlobId,

    pub start_byte: usize,

    pub end_byte: usize,

    /// The matched content of all the matches in the group
    pub groups: Groups,

    /// The assigned status
    pub status: Option<Status>,

    /// The assigned comment
    pub comment: Option<String>,
}

impl Annotation {
    pub fn check_valid(&self) -> Result<()> {
        // TODO: check that the given finding ID matches the computed one
        // TODO: check that the given match ID matches the computed one
        // TODO: check that start_byte < end_byte
        // TODO: check that at least one of status and comment are given
        // TODO: check that groups is nonempty
        // TODO: check that rule_structural_id has the correct format (40-character hex string)

        todo!();
    }
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
