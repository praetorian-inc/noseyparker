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

/// No JSONL format support for the `rules list` command
#[test]
fn rules_list_jsonl() {
    assert_cmd_snapshot!(noseyparker_failure!("rules", "list", "-f", "jsonl"));
}

/// Check the default rules using the built-in linter.
#[test]
fn rules_check_builtins() {
    assert_cmd_snapshot!(noseyparker_success!("rules", "check", "--warnings-as-errors"));
}

/// Check that the `rules list --builtins false` option works as expected
#[test]
fn rules_list_no_builtins() {
    assert_cmd_snapshot!(noseyparker_success!("rules", "list", "--load-builtins=false"));
}
