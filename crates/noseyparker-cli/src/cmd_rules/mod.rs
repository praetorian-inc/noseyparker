use anyhow::Result;

mod cmd_rules_check;
mod cmd_rules_list;
use crate::args;

pub fn run(global_args: &args::GlobalArgs, args: &args::RulesArgs) -> Result<()> {
    match &args.command {
        args::RulesCommand::Check(args) => cmd_rules_check::run(global_args, args),
        args::RulesCommand::List(args) => cmd_rules_list::run(global_args, args),
    }
}
