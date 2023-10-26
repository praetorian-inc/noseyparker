//! Tests for Nosey Parker's `rules` command

use super::*;

/// Check the default list of rules in Nosey Parker using a snapshot test.
/// This will alert us to when the default rules have changed for some reason (usually because a
/// rule has been added).
#[test]
fn rules_list_noargs() {
    assert_cmd_snapshot!(noseyparker_success!("rules", "list"));
}

#[test]
fn rules_list_json() {
    assert_cmd_snapshot!(noseyparker_success!("rules", "list", "--format=json"));
}

#[test]
fn rules_list_jsonl() {
    assert_cmd_snapshot!(noseyparker_success!("rules", "list", "-f", "jsonl"));
}
