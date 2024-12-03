//! Tests for Nosey Parker `generate` functionality

use super::*;

#[test]
fn generate_json_schema() {
    let cmd = noseyparker_success!("generate", "json-schema");

    let output = cmd.get_output();
    let status = output.status;
    assert!(status.success());
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_json_snapshot!(stdout);
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert_eq!(stderr, "");
}
