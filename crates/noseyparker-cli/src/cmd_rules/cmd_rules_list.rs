use anyhow::{Context, Result};
use noseyparker::defaults::get_default_rules;
use noseyparker_rules::{Rule, Rules};
use serde::Serialize;
use tracing::debug_span;

use crate::args::{GlobalArgs, Reportable, RulesListArgs, RulesListOutputFormat};

pub fn run(_global_args: &GlobalArgs, args: &RulesListArgs) -> Result<()> {
    let _span = debug_span!("cmd_rules_list").entered();

    let output = args
        .output_args
        .get_writer()
        .context("Failed to get output writer")?;

    // XXX should factor out rule loading code into one place; this duplicates cmd_scan
    let rules = {
        let mut rules = get_default_rules().context("Failed to load default rules")?;
        if !args.rules.is_empty() {
            let custom_rules =
                Rules::from_paths(&args.rules).context("Failed to load specified rules files")?;
            rules.extend(custom_rules);
        }
        rules.rules.sort_by_key(|r| r.id.clone());
        rules
    };

    let reporter = RulesReporter { rules };
    reporter.report(args.output_args.format, output)
}

struct RulesReporter {
    rules: Rules,
}

impl Reportable for RulesReporter {
    type Format = RulesListOutputFormat;

    fn report<W: std::io::Write>(&self, format: Self::Format, writer: W) -> Result<()> {
        match format {
            RulesListOutputFormat::Human => self.human_format(writer),
            RulesListOutputFormat::Json => self.json_format(writer),
            RulesListOutputFormat::Jsonl => self.jsonl_format(writer),
        }
    }
}

impl RulesReporter {
    fn table(&self) -> prettytable::Table {
        use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
        use prettytable::row;

        let f = FormatBuilder::new()
            // .column_separator('│')
            // .separators(&[LinePosition::Title], LineSeparator::new('─', '┼', '├', '┤'))
            .column_separator(' ')
            .separators(&[LinePosition::Title], LineSeparator::new('─', '─', '─', '─'))
            .padding(1, 1)
            .build();

        let mut table: prettytable::Table = self
            .rules
            .iter()
            .map(|r| {
                row![
                     r -> &r.id,
                     l -> &r.name,
                ]
            })
            .collect();
        table.set_format(f);
        table.set_titles(row![rb -> "ID", lb -> "Name"]);
        table
    }

    fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        writeln!(writer)?;
        let table = self.table();
        table.print(&mut writer)?;
        Ok(())
    }

    fn json_format<W: std::io::Write>(&self, writer: W) -> Result<()> {
        let entries: Vec<_> = self.rules.iter().map(|r| RulesListEntry::new(r)).collect();
        serde_json::to_writer_pretty(writer, &entries)?;
        Ok(())
    }

    fn jsonl_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        for rule in self.rules.iter() {
            let entry = RulesListEntry::new(&rule);
            serde_json::to_writer(&mut writer, &entry)?;
            writeln!(&mut writer)?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct RulesListEntry<'r> {
    id: &'r str,
    name: &'r str,
}

impl <'r> RulesListEntry<'r> {
    pub fn new(rule: &'r Rule) -> Self {
        Self {
            id: &rule.id,
            name: &rule.name,
        }
    }
}
