use crate::blob_id::BlobId;
use crate::location::{LocationMapping, OffsetSpan, SourceSpan};
use crate::matcher::BlobMatch;
use crate::provenance::Provenance;
use crate::utils::decode_utf8_lossy_escape;

use indenter::indented;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Write};

// -------------------------------------------------------------------------------------------------
// Match
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Match {
    /// The blob this match comes from
    pub blob_id: BlobId,

    /// The location of the matching input, as byte offsets
    pub matching_input_offset_span: OffsetSpan,

    /// The location of the matching input, as line and column number
    pub matching_input_source_span: SourceSpan,

    /// The matching input
    pub matching_input: Vec<u8>,

    /// A snippet of the input immediately prior to `matching_input`
    pub before_snippet: Vec<u8>,

    /// A snippet of the input immediately after `matching_input`
    pub after_snippet: Vec<u8>,

    /// The capture group number, indexed from 1
    pub group_index: u8,

    /// The capture group
    pub group_input: Vec<u8>,

    /// The rule that produced this match
    pub rule_name: String,

    /// Where did this blob come from?
    pub provenance: Provenance,
}

impl Match {
    #[inline]
    pub fn new<'r, 'b>(
        loc_mapping: &LocationMapping,
        blob_match: BlobMatch<'r, 'b>,
        provenance: &Provenance,
    ) -> Vec<Self> {
        let offsets = &blob_match.matching_input_offset_span;

        const SNIPPET_CONTEXT_BYTES: usize = 128; // FIXME:parameterize this and expose to CLI

        // FIXME: have the snippets start from a line break in the input when feasible, and include an ellipsis otherwise to indicate truncation
        let start = offsets.start.saturating_sub(SNIPPET_CONTEXT_BYTES);
        let end = offsets.start;
        let before_snippet = &blob_match.blob.bytes[start..end];

        let start = offsets.end;
        let end = offsets
            .end
            .saturating_add(SNIPPET_CONTEXT_BYTES)
            .min(blob_match.blob.len());
        let after_snippet = &blob_match.blob.bytes[start..end];
        let blob_id = &blob_match.blob.id;
        let rule_name = &blob_match.rule.name;
        let matching_input = blob_match.matching_input;
        let source_span = loc_mapping.get_source_span(offsets);

        blob_match
            .captures
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(group_index, group)| {
                let group = group?; // XXX should we warn on empty match groups?
                Some(Match {
                    blob_id: blob_id.clone(),
                    rule_name: rule_name.clone(),
                    matching_input: matching_input.to_owned(),
                    matching_input_offset_span: offsets.clone(),
                    matching_input_source_span: source_span.clone(),
                    group_input: group.as_bytes().to_owned(),
                    group_index: group_index
                        .try_into()
                        .expect("group index should fit in u8"),
                    provenance: provenance.clone(),
                    before_snippet: before_snippet.to_owned(),
                    after_snippet: after_snippet.to_owned(),
                })
            })
            .collect()
    }

    pub fn snippet(&self) -> String {
        let snippet: Vec<u8> = [
            self.before_snippet.as_slice(),
            self.matching_input.as_slice(),
            self.after_snippet.as_slice(),
        ]
        .concat();
        decode_utf8_lossy_escape(&snippet)
    }
}

impl Display for Match {
    /// Render this finding human-readable style to the given formatter.
    ///
    /// Note: that the output emitted spans multiple lines.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Rule: {}", &self.rule_name)?;
        match &self.provenance {
            Provenance::FromFile(p) => {
                writeln!(f, "File: {}", p.to_string_lossy())?;
            }
            Provenance::FromGitRepo(p) => {
                writeln!(f, "Git repo: {}", p.to_string_lossy())?;
                writeln!(f, "Blob: {}", &self.blob_id)?;
            }
        }
        writeln!(f, "Lines: {}", &self.matching_input_source_span)?;
        writeln!(f, "Match: {:?}", decode_utf8_lossy_escape(&self.group_input))?;
        writeln!(f, "Snippet:\n")?;
        let mut f = indented(f).with_str("    ");
        writeln!(f, "{}", &self.snippet())?;

        Ok(())
    }
}
