use ratatui::Frame;

use crate::app::{App, LineType, Mode};
use crate::widgets::confirm_prompt::ConfirmPrompt;

/// Render a confirmation prompt for the current app state
pub fn render_confirmation_prompt(app: &App, frame: &mut Frame) {
    match app.mode {
        Mode::Rename => {
            // Determine if we're renaming a window or session
            let (title, message) = if let Some(line) = app.tree_lines.get(app.selected_index) {
                match line.line_type {
                    LineType::Window => ("Rename Window", "Enter new window name:"),
                    LineType::Session => ("Rename Session", "Enter new session name:"),
                }
            } else {
                ("Rename", "Enter new name:")
            };

            let prompt = ConfirmPrompt::new(title, message)
                .input(&app.popup_input)
                .show_cursor(true)
                .border_style(app.config.colors.popup_border.to_ratatui_style())
                .text_style(app.config.colors.popup_text.to_ratatui_style())
                .input_style(app.config.colors.popup_input.to_ratatui_style());

            prompt.render(frame, frame.area());
        }
        Mode::DeleteConfirm => {
            // Determine if we're deleting a window or session
            let (title, message) = if let Some(line) = app.tree_lines.get(app.selected_index) {
                match line.line_type {
                    LineType::Window => {
                        let window_name = line.window.as_ref().map_or("window", |w| &w.name);
                        (
                            "Delete Window",
                            format!(
                                "Are you sure you want to delete window '{window_name}'? (y/n)"
                            ),
                        )
                    }
                    LineType::Session => {
                        let session_name_str = match &line.session_name {
                            Some(name) => name.clone(),
                            None => String::from("session"),
                        };
                        (
                            "Delete Session",
                            format!(
                                "Are you sure you want to delete session '{session_name_str}'? (y/n)"
                            ),
                        )
                    }
                }
            } else {
                (
                    "Delete",
                    String::from("Are you sure you want to delete this item? (y/n)"),
                )
            };

            let prompt = ConfirmPrompt::new(title, &message)
                .border_style(app.config.colors.popup_border.to_ratatui_style())
                .text_style(app.config.colors.popup_text.to_ratatui_style());

            prompt.render(frame, frame.area());
        }
        _ => {
            // No confirmation prompt for other modes
        }
    }
}
