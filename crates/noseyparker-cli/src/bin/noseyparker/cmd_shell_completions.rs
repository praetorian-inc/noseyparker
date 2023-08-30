use crate::args::{GlobalArgs, ShellFormat, ShellCompletionsArgs, CommandLineArgs};
use anyhow::Result;
use clap::{CommandFactory, Command};
use clap_complete::{generate, shells::Bash, shells::Zsh, shells::Fish, shells::PowerShell, shells::Elvish};

pub fn run(_global_args: &GlobalArgs, args: &ShellCompletionsArgs) -> Result<()> {
    let mut cmd = CommandLineArgs::command();
    generate_completions_for_shell(&args.shell, &mut cmd)
}

fn generate_completions_for_shell(shell: &ShellFormat, cmd: &mut Command) -> Result<()> {
    let bin_name = "noseyparker";
    let std_out = &mut std::io::stdout();

    match shell {
        ShellFormat::Bash => generate(Bash, cmd, bin_name, std_out),
        ShellFormat::Zsh => generate(Zsh, cmd, bin_name, std_out),
        ShellFormat::Fish => generate(Fish, cmd, bin_name, std_out),
        ShellFormat::PowerShell => generate(PowerShell, cmd, bin_name, std_out),
        ShellFormat::Elvish => generate(Elvish, cmd, bin_name, std_out),
    }

    Ok(())
}
