use ratatui::{
    prelude::*,
    widgets::{Block, Paragraph},
};

/// A struct representing a help item with a key and its description
pub struct HelpItem<'a> {
    pub key: &'a str,
    pub description: &'a str,
}

/// The HelpPanel widget that displays keyboard shortcuts
pub struct HelpPanel<'a> {
    /// List of help items to display
    items: Vec<HelpItem<'a>>,
    /// Block to wrap the widget in
    block: Option<Block<'a>>,
    /// Key style
    key_style: Style,
    /// Description style
    desc_style: Style,
}

impl<'a> HelpPanel<'a> {
    /// Create a new HelpPanel with the given items
    pub fn new(items: Vec<HelpItem<'a>>) -> Self {
        Self {
            items,
            block: None,
            key_style: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            desc_style: Style::default().fg(Color::Gray),
        }
    }

    /// Set the key style
    pub fn key_style(mut self, style: Style) -> Self {
        self.key_style = style;
        self
    }

    /// Set the description style
    pub fn desc_style(mut self, style: Style) -> Self {
        self.desc_style = style;
        self
    }
}

impl<'a> Widget for HelpPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block.unwrap_or_default();
        let inner_area = block.inner(area);

        // Render the block if it exists
        block.render(area, buf);

        // Create spans for each help item
        let mut spans = Vec::new();

        // Add a space at the beginning
        spans.push(Span::raw(" "));

        for (i, item) in self.items.iter().enumerate() {
            // Add the key with styling
            spans.push(Span::styled(item.key, self.key_style));

            // Add the description
            spans.push(Span::styled(
                format!(":{} ", item.description),
                self.desc_style,
            ));

            // Add a separator between items (except for the last one)
            if i < self.items.len() - 1 {
                spans.push(Span::raw(" "));
            }
        }

        // Create a paragraph with all the spans
        let help_text = Paragraph::new(Line::from(spans)).alignment(Alignment::Left);

        // Render the help text
        help_text.render(inner_area, buf);
    }
}
