use ratatui::{
    layout::{Margin, Rect},
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// A generic confirmation prompt widget
pub struct ConfirmPrompt<'a> {
    /// Title of the confirmation prompt
    title: &'a str,
    /// Message to display in the prompt
    message: &'a str,
    /// Input text (if any)
    input: Option<&'a str>,
    /// Whether to show the cursor
    show_cursor: bool,
    /// Border style
    border_style: Style,
    /// Text style
    text_style: Style,
    /// Input style
    input_style: Style,
}

impl<'a> ConfirmPrompt<'a> {
    /// Create a new confirmation prompt with the given title and message
    pub fn new(title: &'a str, message: &'a str) -> Self {
        Self {
            title,
            message,
            input: None,
            show_cursor: false,
            border_style: Style::default(),
            text_style: Style::default(),
            input_style: Style::default(),
        }
    }

    /// Set the input text
    pub fn input(mut self, input: &'a str) -> Self {
        self.input = Some(input);
        self
    }

    /// Set whether to show the cursor
    pub fn show_cursor(mut self, show: bool) -> Self {
        self.show_cursor = show;
        self
    }

    /// Set the border style
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Set the text style
    pub fn text_style(mut self, style: Style) -> Self {
        self.text_style = style;
        self
    }

    /// Set the input style
    pub fn input_style(mut self, style: Style) -> Self {
        self.input_style = style;
        self
    }

    /// Render the confirmation prompt
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Create a centered popup area
        let popup_area = self.centered_rect(50, 20, area);

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(self.border_style);

        frame.render_widget(popup_block, popup_area);

        // Calculate the inner area for content
        let inner_area = popup_area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        // Render the message
        let message_paragraph = Paragraph::new(self.message)
            .style(self.text_style)
            .wrap(Wrap { trim: true });

        let message_height = 1; // Assuming message is a single line
        let message_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: inner_area.width,
            height: message_height,
        };

        frame.render_widget(message_paragraph, message_area);

        // If there's input, render it below the message
        if let Some(input) = self.input {
            let input_paragraph = Paragraph::new(input)
                .style(self.input_style)
                .wrap(Wrap { trim: true });

            let input_area = Rect {
                x: inner_area.x,
                y: inner_area.y + message_height + 1, // +1 for spacing
                width: inner_area.width,
                height: 1,
            };

            frame.render_widget(input_paragraph, input_area);

            // Position cursor at end of input if needed
            if self.show_cursor {
                frame.set_cursor_position((input_area.x + input.len() as u16, input_area.y));
            }
        }
    }

    /// Helper function to create a centered rectangle
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}
