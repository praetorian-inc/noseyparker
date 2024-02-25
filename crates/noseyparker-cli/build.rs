/// This build script uses `vergen` to expose lots of build information at compile time.
/// This information is used in the `noseyparker` CLI in its `version/-V/--version` commands.
use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    // Emit the instructions
    EmitBuilder::builder()
        .all_build()
        .all_git()
        .all_cargo()
        .all_rustc()
        .all_sysinfo()
        .emit()?;
    Ok(())
}
