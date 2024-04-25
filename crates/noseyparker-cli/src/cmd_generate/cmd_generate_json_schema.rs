use anyhow::Result;
use tracing::info;

use crate::args::{GlobalArgs, JsonSchemaArgs};
use crate::cmd_report::Finding;
use crate::util::get_writer_for_file_or_stdout;

pub fn run(_global_args: &GlobalArgs, args: &JsonSchemaArgs) -> Result<()> {
    let schema = schemars::schema_for!(Vec<Finding>);

    let mut writer = get_writer_for_file_or_stdout(args.output.as_ref())?;
    writeln!(writer, "{}", serde_json::to_string_pretty(&schema).unwrap())?;
    if let Some(output) = &args.output {
        info!("Wrote JSON schema to {}", output.display());
    }
    Ok(())
}
