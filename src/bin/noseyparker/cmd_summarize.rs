use anyhow::{Context, Result};
use indicatif::HumanCount;

use noseyparker::datastore::{Datastore, MatchSummary};

use crate::args;

pub fn run(_global_args: &args::GlobalArgs, args: &args::SummarizeArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;
    let mut writer = args
        .output_args
        .get_writer()
        .context("Failed to open output destination for writing")?;
    let summary = datastore.summarize()?;

    let run_inner = move || -> std::io::Result<()> {
        match &args.output_args.format {
            args::OutputFormat::Human => {
                writeln!(writer)?;
                let table = summary_table(summary);
                // FIXME: this doesn't preserve ANSI styling on the table
                table.print(&mut writer)?;
            }
            args::OutputFormat::Json => {
                serde_json::to_writer_pretty(&mut writer, &summary)?;
            }
            args::OutputFormat::Jsonl => {
                for entry in summary.0.iter() {
                    serde_json::to_writer(&mut writer, entry)?;
                    writeln!(&mut writer)?;
                }
            }
        }
        Ok(())
    };
    match run_inner() {
        // Ignore SIGPIPE errors, like those that can come from piping to `head`
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => { Ok(()) }
        Err(e) => Err(e)?,
        Ok(()) => Ok(()),
    }
}

pub fn summary_table(summary: MatchSummary) -> prettytable::Table {
    use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
    use prettytable::row;

    let f = FormatBuilder::new()
        // .column_separator('│')
        // .separators(&[LinePosition::Title], LineSeparator::new('─', '┼', '├', '┤'))
        .column_separator(' ')
        .separators(&[LinePosition::Title], LineSeparator::new('─', '─', '─', '─'))
        .padding(1, 1)
        .build();

    let mut table: prettytable::Table = summary
        .0
        .into_iter()
        .map(|e| row![
             l -> &e.rule_name,
             r -> HumanCount(e.distinct_count.try_into().unwrap()),
             r -> HumanCount(e.total_count.try_into().unwrap())
        ])
        .collect();
    table.set_format(f);
    table.set_titles(row![lb -> "Rule", cb -> "Distinct Matches", cb -> "Total Matches"]);
    table
}
