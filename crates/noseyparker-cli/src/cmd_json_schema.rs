use crate::args::{GlobalArgs, JsonSchemaArgs};
use crate::cmd_report::Finding;

use anyhow::Result;

pub fn run(_global_args: &GlobalArgs, _args: &JsonSchemaArgs) -> Result<()> {
    let schema = schemars::schema_for!(Vec<Finding>);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
    return Ok(());
}
