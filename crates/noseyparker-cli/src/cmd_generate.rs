use anyhow::Result;

use crate::args::{GenerateArgs, GenerateCommand, GlobalArgs};

mod cmd_generate_json_schema;
mod cmd_generate_manpages;
mod cmd_generate_shell_completions;

pub fn run(global_args: &GlobalArgs, args: &GenerateArgs) -> Result<()> {
    match &args.command {
        GenerateCommand::ShellCompletions(args) => {
            cmd_generate_shell_completions::run(global_args, args)
        }
        GenerateCommand::JsonSchema(args) => cmd_generate_json_schema::run(global_args, args),
        GenerateCommand::ManPages(args) => cmd_generate_manpages::run(global_args, args),
    }
}
