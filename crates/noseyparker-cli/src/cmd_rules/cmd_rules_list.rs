use anyhow::{Context, Result};
use noseyparker_rules::{Rule, RuleSyntax, RulesetSyntax};
use serde::Serialize;
use tracing::debug_span;

use crate::args::{GlobalArgs, RulesListArgs, RulesListOutputFormat};
use crate::reportable::Reportable;
use crate::rule_loader::{LoadedRules, RuleLoader};

pub fn run(_global_args: &GlobalArgs, args: &RulesListArgs) -> Result<()> {
    let _span = debug_span!("cmd_rules_list").entered();

    let output = args
        .output_args
        .get_writer()
        .context("Failed to get output writer")?;

    let loaded = RuleLoader::from_rule_specifiers(&args.rules)
        .load()
        .context("Failed to load rules")?;

    let reporter = RulesReporter { loaded };
    reporter.report(args.output_args.format, output)
}

struct RulesReporter {
    loaded: LoadedRules,
}

impl Reportable for RulesReporter {
    type Format = RulesListOutputFormat;

    fn report<W: std::io::Write>(&self, format: Self::Format, writer: W) -> Result<()> {
        match format {
            RulesListOutputFormat::Human => self.human_format(writer),
            RulesListOutputFormat::Json => self.json_format(writer),
        }
    }
}

impl RulesReporter {
    fn get_entries(&self) -> Entries<'_> {
        let mut rules: Vec<_> = self.loaded.iter_rules().map(RuleEntry::new).collect();
        rules.sort_by(|r1, r2| r1.id.cmp(r2.id));

        let mut rulesets: Vec<_> = self.loaded.iter_rulesets().map(RulesetEntry::new).collect();
        rulesets.sort_by(|r1, r2| r1.id.cmp(r2.id));

        Entries { rules, rulesets }
    }

    fn human_format<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        let entries = self.get_entries();

        let rules_table = entries.rules_table();
        writeln!(writer)?;
        rules_table.print(&mut writer)?;

        let rulesets_table = entries.rulesets_table();
        writeln!(writer)?;
        rulesets_table.print(&mut writer)?;

        Ok(())
    }

    fn json_format<W: std::io::Write>(&self, writer: W) -> Result<()> {
        let entries = self.get_entries();
        serde_json::to_writer_pretty(writer, &entries)?;
        Ok(())
    }
}

#[derive(Serialize)]
struct Entries<'r> {
    rules: Vec<RuleEntry<'r>>,
    rulesets: Vec<RulesetEntry<'r>>,
}

#[derive(Serialize)]
struct RuleEntry<'r> {
    id: &'r str,
    structural_id: &'r str,
    name: &'r str,
    syntax: &'r RuleSyntax,
}

impl<'r> RuleEntry<'r> {
    pub fn new(rule: &'r Rule) -> Self {
        Self {
            id: rule.id(),
            name: rule.name(),
            structural_id: rule.structural_id(),
            syntax: rule.syntax(),
        }
    }
}

#[derive(Serialize)]
struct RulesetEntry<'r> {
    id: &'r str,
    name: &'r str,
    num_rules: usize,
}

impl<'r> RulesetEntry<'r> {
    pub fn new(ruleset: &'r RulesetSyntax) -> Self {
        Self {
            id: &ruleset.id,
            name: &ruleset.name,
            num_rules: ruleset.num_rules(),
        }
    }
}

impl<'r> Entries<'r> {
    fn rules_table(&self) -> prettytable::Table {
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
                let mut cats = r.syntax.categories.clone();
                cats.sort();
                let cats: String = cats.join(", ");
                row![l -> &r.id, l -> &r.name, l -> cats]
            })
            .collect();
        table.set_format(f);
        table.set_titles(row![lb -> "Rule ID", lb -> "Rule Name", lb -> "Categories"]);
        table
    }

    fn rulesets_table(&self) -> prettytable::Table {
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
            .rulesets
            .iter()
            .map(|r| row![l -> &r.id, l -> &r.name, r -> r.num_rules])
            .collect();
        table.set_format(f);
        table.set_titles(row![lb -> "Ruleset ID", lb -> "Ruleset Name", rb -> "Rules"]);
        table
    }
}
