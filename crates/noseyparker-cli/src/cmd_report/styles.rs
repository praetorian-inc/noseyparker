pub use console::{Style, StyledObject};

pub struct Styles {
    pub style_finding_heading: Style,
    pub style_rule: Style,
    pub style_heading: Style,
    pub style_match: Style,
    pub style_metadata: Style,
    pub style_id: Style,
}

impl Styles {
    pub fn new(styles_enabled: bool) -> Self {
        let style_finding_heading = Style::new()
            .bold()
            .bright()
            .white()
            .force_styling(styles_enabled);
        let style_rule = Style::new()
            .bright()
            .bold()
            .blue()
            .force_styling(styles_enabled);
        let style_heading = Style::new().bold().force_styling(styles_enabled);
        let style_match = Style::new().yellow().force_styling(styles_enabled);
        let style_metadata = Style::new().bright().blue().force_styling(styles_enabled);
        let style_id = Style::new().bright().green().force_styling(styles_enabled);

        Self {
            style_finding_heading,
            style_rule,
            style_heading,
            style_match,
            style_metadata,
            style_id,
        }
    }
}
