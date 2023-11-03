use anyhow::{Context, Result};
use indicatif::HumanCount;

use noseyparker::datastore::{Datastore, MatchSummary};

use crate::args::{GlobalArgs, SummarizeArgs, SummarizeOutputFormat};
use crate::reportable::Reportable;

struct MatchSummaryReporter(MatchSummary);

impl Reportable for MatchSummaryReporter {
    type Format = SummarizeOutputFormat;

    fn report<W: std::io::Write>(&self, format: Self::Format, writer: W) -> Result<()> {
        match format {
            SummarizeOutputFormat::Human => self.human_format(writer),
            SummarizeOutputFormat::Json => self.json_format(writer),
            SummarizeOutputFormat::Jsonl => self.jsonl_format(writer),
        }
    }
}

impl MatchSummaryReporter {
    fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let summary = &self.0;
        writeln!(writer)?;
        let table = summary_table(summary);
        // FIXME: this doesn't preserve ANSI styling on the table
        table.print(&mut writer)?;
        Ok(())
    }

    fn json_format<W: std::io::Write>(&self, writer: W) -> Result<()> {
        let summary = &self.0;
        serde_json::to_writer_pretty(writer, &summary)?;
        Ok(())
    }

    fn jsonl_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let summary = &self.0;
        for entry in summary.0.iter() {
            serde_json::to_writer(&mut writer, entry)?;
            writeln!(&mut writer)?;
        }
        Ok(())
    }
}

pub fn run(global_args: &GlobalArgs, args: &SummarizeArgs) -> Result<()> {
    let datastore = Datastore::open(&args.datastore, global_args.advanced.sqlite_cache_size)
        .with_context(|| format!("Failed to open datastore at {}", args.datastore.display()))?;
    let output = args
        .output_args
        .get_writer()
        .context("Failed to get output writer")?;
    MatchSummaryReporter(datastore.summarize()?).report(args.output_args.format, output)
}

pub fn summary_table(summary: &MatchSummary) -> prettytable::Table {
    use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
    use prettytable::row;

    let f = FormatBuilder::new()
        .column_separator(' ')
        .separators(&[LinePosition::Title], LineSeparator::new('─', '─', '─', '─'))
        .padding(1, 1)
        .build();

    let mut table: prettytable::Table = summary
        .0
        .iter()
        .map(|e| {
            row![
                 l -> &e.rule_name,
                 r -> HumanCount(e.distinct_count.try_into().unwrap()),
                 r -> HumanCount(e.total_count.try_into().unwrap())
            ]
        })
        .collect();
    table.set_format(f);
    table.set_titles(row![lb -> "Rule", cb -> "Total Findings", cb -> "Total Matches"]);
    table
}
