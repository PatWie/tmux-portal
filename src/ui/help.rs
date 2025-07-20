use ratatui::prelude::*;

use crate::app::{App, Mode};
use crate::widgets::help_panel::{HelpItem, HelpPanel};

/// Get the appropriate help items based on the current app mode
pub fn get_help_items_for_mode(mode: &Mode) -> Vec<HelpItem<'static>> {
    match mode {
        Mode::Window => vec![
            HelpItem {
                key: "q",
                description: "quit",
            },
            HelpItem {
                key: "j/k",
                description: "move",
            },
            HelpItem {
                key: "Enter",
                description: "select",
            },
            HelpItem {
                key: "r/,",
                description: "rename",
            },
            HelpItem {
                key: "x",
                description: "delete",
            },
            HelpItem {
                key: "R",
                description: "refresh",
            },
            HelpItem {
                key: "/",
                description: "search",
            },
            HelpItem {
                key: "F",
                description: "find project",
            },
            HelpItem {
                key: "S",
                description: "session mode",
            },
            HelpItem {
                key: "J/K",
                description: "move item",
            },
            HelpItem {
                key: "C",
                description: "create window",
            },
        ],
        Mode::Rename => vec![
            HelpItem {
                key: "Esc",
                description: "cancel",
            },
            HelpItem {
                key: "Enter",
                description: "confirm",
            },
        ],
        Mode::Search => vec![
            HelpItem {
                key: "Esc",
                description: "cancel",
            },
            HelpItem {
                key: "Enter",
                description: "select",
            },
            HelpItem {
                key: "↑/↓",
                description: "navigate",
            },
        ],
        Mode::QuickSearch => vec![
            HelpItem {
                key: "Esc",
                description: "cancel",
            },
            HelpItem {
                key: "Enter",
                description: "select",
            },
            HelpItem {
                key: "↑/↓",
                description: "navigate",
            },
        ],
        Mode::Session => vec![
            HelpItem {
                key: "q/Esc",
                description: "normal mode",
            },
            HelpItem {
                key: "j/k",
                description: "move between sessions",
            },
            HelpItem {
                key: "Enter",
                description: "switch to session",
            },
            HelpItem {
                key: "/r",
                description: "rename session",
            },
            HelpItem {
                key: "x",
                description: "delete session",
            },
            HelpItem {
                key: "J/K",
                description: "move session",
            },
        ],
        Mode::DeleteConfirm => vec![
            HelpItem {
                key: "y",
                description: "confirm delete",
            },
            HelpItem {
                key: "n/Esc",
                description: "cancel",
            },
        ],
    }
}

/// Render the help panel for the current app state
pub fn render_help_panel(app: &App, area: Rect, buf: &mut Buffer) {
    let help_items = get_help_items_for_mode(&app.mode);

    let help_panel = HelpPanel::new(help_items)
        .key_style(app.config.colors.help_key.to_ratatui_style())
        .desc_style(app.config.colors.help_text.to_ratatui_style());

    help_panel.render(area, buf);
}
