use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::ui::confirm::render_confirmation_prompt;
use crate::ui::help::render_help_panel;
use crate::{
    app::{App, LineType, Mode},
    config::ColorConfig,
};

// Helper function to get the display text for a mode
fn get_mode_text(mode: &Mode) -> &'static str {
    match mode {
        Mode::Window => "-- WINDOW --",
        Mode::Rename => "-- RENAME --",
        Mode::Search => "-- SEARCH --",
        Mode::QuickSearch => "-- QUICK --",
        Mode::Session => "-- SESSION --",
        Mode::DeleteConfirm => "-- CONFIRM --",
    }
}
fn get_mode_style(mode: &Mode, colors: &ColorConfig) -> Style {
    match mode {
        Mode::Window => colors.border_list.to_ratatui_style(),
        Mode::Rename => colors.border_prompt.to_ratatui_style(),
        Mode::Search | Mode::QuickSearch => colors.border_search.to_ratatui_style(),
        Mode::Session => colors.border_list.to_ratatui_style(), // TODO: Add session mode color
        Mode::DeleteConfirm => colors.border_prompt.to_ratatui_style(), // Use insert color for delete confirmation
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.mode {
        Mode::Search => {
            draw_search_interface(f, app);
        }
        Mode::QuickSearch => {
            draw_quick_search_interface(f, app);
        }
        Mode::Session => {
            draw_session_mode_interface(f, app);
        }
        _ => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.area());

            // Update scroll offset based on current viewport size
            app.update_scroll_offset(chunks[0].height as usize);

            draw_main_content(f, app, chunks[0]);
            draw_status_bar(f, app, chunks[1]);

            if app.show_popup {
                render_confirmation_prompt(app, f);
            }
        }
    }
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    let line_numbers = app.get_window_line_numbers();
    let mut items = Vec::new();

    // Always use 3 characters for line numbers
    let line_number_width = 3;

    // Calculate the visible range based on scroll offset
    let viewport_height = area.height as usize;
    let start_idx = app.scroll_offset;
    let end_idx = (start_idx + viewport_height).min(app.tree_lines.len());

    for i in start_idx..end_idx {
        let tree_line = &app.tree_lines[i];
        let is_selected = i == app.selected_index;

        let (line_number_str, line_number_style) = if tree_line.line_type == LineType::Window {
            if let Some(&relative_num) = line_numbers.get(&i) {
                if relative_num == 0 {
                    let padding = " ".repeat(app.config.line_numbers.padding);
                    (
                        format!("{:<width$}{}", "0", padding, width = line_number_width),
                        app.config
                            .line_numbers
                            .current_line_color
                            .to_ratatui_style(),
                    )
                } else {
                    let padding = " ".repeat(app.config.line_numbers.padding);
                    (
                        format!(
                            "{:>width$}{}",
                            relative_num.abs(),
                            padding,
                            width = line_number_width
                        ),
                        app.config.line_numbers.other_lines_color.to_ratatui_style(),
                    )
                }
            } else {
                let total_width = line_number_width + app.config.line_numbers.padding;
                (" ".repeat(total_width), Style::default())
            }
        } else {
            let total_width = line_number_width + app.config.line_numbers.padding;
            (" ".repeat(total_width), Style::default())
        };

        // Use the content as-is since it already contains tree drawing characters
        let display_content = tree_line.content.clone();

        let content_spans = vec![
            Span::styled(line_number_str, line_number_style),
            Span::raw(display_content),
        ];

        let style = match tree_line.line_type {
            LineType::Session => {
                if is_selected {
                    app.config.colors.session_selected.to_ratatui_style()
                } else {
                    app.config.colors.session.to_ratatui_style()
                }
            }
            LineType::Window => {
                if is_selected {
                    app.config.colors.window_selected.to_ratatui_style()
                } else if tree_line.window.as_ref().is_some_and(|w| w.active) {
                    app.config.colors.window_active.to_ratatui_style()
                } else {
                    app.config.colors.window_inactive.to_ratatui_style()
                }
            }
        };

        items.push(ListItem::new(Line::from(content_spans)).style(style));
    }

    let list = List::new(items);

    let mut list_state = ListState::default();
    if app.selected_index >= start_idx && app.selected_index < end_idx {
        list_state.select(Some(app.selected_index - start_idx));
    }

    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14), // Mode indicator
            Constraint::Min(10),    // Help text - give it more space
            Constraint::Length(30), // Stats/error
        ])
        .split(area);

    // Mode indicator (left)
    let mode_text = get_mode_text(&app.mode);
    let mode_style = get_mode_style(&app.mode, &app.config.colors);

    let mode_paragraph = Paragraph::new(format!(" {mode_text}")).style(mode_style);

    f.render_widget(mode_paragraph, status_chunks[0]);

    // Help text (center) - using our new help panel widget
    render_help_panel(app, status_chunks[1], f.buffer_mut());

    // Numeric buffer display
    if !app.numeric_buffer.is_empty() {
        let numeric_text = Paragraph::new(format!(" [{}]", app.numeric_buffer))
            .style(app.config.colors.numeric_buffer.to_ratatui_style());

        // Create a small area at the right side of the help panel for the numeric buffer
        let numeric_area = Rect {
            x: status_chunks[1].x + status_chunks[1].width - 10,
            y: status_chunks[1].y,
            width: 10,
            height: 1,
        };

        f.render_widget(numeric_text, numeric_area);
    }

    // Error message or session count (right)
    let right_content = if let Some(error) = &app.error_message {
        Paragraph::new(format!(" {error}")).style(app.config.colors.error_text.to_ratatui_style())
    } else {
        let session_count = app.sessions.len();
        let window_count: usize = app.sessions.iter().map(|s| s.windows.len()).sum();
        Paragraph::new(format!(
            " Sessions: {session_count} | Windows: {window_count}"
        ))
        .style(app.config.colors.status_text.to_ratatui_style())
    };

    f.render_widget(right_content, status_chunks[2]);
}

fn draw_session_mode_interface(f: &mut Frame, app: &App) {
    // Use the same layout as normal mode but with session mode indicators
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    draw_main_content(f, app, chunks[0]);
    draw_status_bar(f, app, chunks[1]);

    if app.show_popup {
        render_confirmation_prompt(app, f);
    }
}

fn draw_search_interface(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input
            Constraint::Min(0),    // Search results
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    // Search input box
    let search_input = Paragraph::new(format!("Search: {}", app.search_query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Fuzzy Search (session/window)")
                .border_style(app.config.colors.border_search.to_ratatui_style()),
        )
        .style(app.config.colors.popup_input.to_ratatui_style());

    f.render_widget(search_input, chunks[0]);

    // Search results
    let results: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let is_selected = i == app.search_selected_index;
            let base_style = if is_selected {
                app.config.colors.window_selected.to_ratatui_style()
            } else {
                app.config.colors.window_inactive.to_ratatui_style()
            };

            let _content_text = format!(
                "{} → {} ({})",
                result.display_text,
                result.session_name,
                result.full_path.display()
            );

            // Create highlighted spans for the display text part
            let highlighted_spans = create_highlighted_spans(
                &result.display_text,
                &result.match_indices,
                base_style,
                app.config.colors.search_highlight.to_ratatui_style(), // Use search_highlight color
            );

            // Add the rest of the content (session and path info)
            let mut all_spans = highlighted_spans;
            all_spans.push(Span::styled(
                format!(
                    " → {} ({})",
                    result.session_name,
                    result.full_path.display()
                ),
                base_style,
            ));

            ListItem::new(Line::from(all_spans))
        })
        .collect();

    let results_list = List::new(results);

    f.render_widget(results_list, chunks[1]);

    // Status bar
    draw_search_status_bar(f, app, chunks[2]);
}

fn create_highlighted_spans<'a>(
    text: &'a str,
    match_indices: &[usize],
    normal_style: Style,
    highlight_style: Style,
) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut current_span = String::new();
    let mut is_highlighted = false;

    for (i, &ch) in chars.iter().enumerate() {
        let should_highlight = match_indices.contains(&i);

        if should_highlight != is_highlighted {
            // Style change - push current span and start new one
            if !current_span.is_empty() {
                let style = if is_highlighted {
                    highlight_style
                } else {
                    normal_style
                };
                spans.push(Span::styled(current_span.clone(), style));
                current_span.clear();
            }
            is_highlighted = should_highlight;
        }

        current_span.push(ch);
    }

    // Push the final span
    if !current_span.is_empty() {
        let style = if is_highlighted {
            highlight_style
        } else {
            normal_style
        };
        spans.push(Span::styled(current_span, style));
    }

    spans
}

fn draw_quick_search_interface(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    // Calculate the area for the tree view, accounting for the search bar height
    let search_bar_height = 3;
    let tree_area = Rect {
        x: chunks[0].x,
        y: chunks[0].y + search_bar_height,
        width: chunks[0].width,
        height: chunks[0].height.saturating_sub(search_bar_height),
    };

    // Draw the main tree view below the search bar
    draw_main_content_with_quick_search_highlights(f, app, tree_area);
    draw_status_bar(f, app, chunks[1]);

    // Draw search input overlay at the top
    let search_area = Rect {
        x: chunks[0].x,
        y: chunks[0].y,
        width: chunks[0].width,
        height: search_bar_height,
    };

    let search_input = Paragraph::new(format!("Search: {}", app.quick_search_query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Quick Search (active sessions/windows)")
                .border_style(app.config.colors.border_search.to_ratatui_style()),
        )
        .style(app.config.colors.popup_input.to_ratatui_style());

    f.render_widget(Clear, search_area); // Clear the background
    f.render_widget(search_input, search_area);
}

fn draw_main_content_with_quick_search_highlights(f: &mut Frame, app: &App, area: Rect) {
    use fuzzy_matcher::FuzzyMatcher;
    use fuzzy_matcher::skim::SkimMatcherV2;

    let line_numbers = app.get_window_line_numbers();
    let mut items = Vec::new();
    let matcher = SkimMatcherV2::default().ignore_case();

    // Always use 3 characters for line numbers
    let line_number_width = 3;

    for (i, tree_line) in app.tree_lines.iter().enumerate() {
        let is_selected = i == app.selected_index;
        let is_quick_search_match = app.quick_search_results.contains(&i);
        let is_quick_search_selected = app
            .quick_search_results
            .get(app.quick_search_selected_index)
            == Some(&i);

        let (line_number_str, line_number_style) = if tree_line.line_type == LineType::Window {
            if let Some(&relative_num) = line_numbers.get(&i) {
                if relative_num == 0 {
                    let padding = " ".repeat(app.config.line_numbers.padding);
                    (
                        format!("{:<width$}{}", "0", padding, width = line_number_width),
                        app.config
                            .line_numbers
                            .current_line_color
                            .to_ratatui_style(),
                    )
                } else {
                    let padding = " ".repeat(app.config.line_numbers.padding);
                    (
                        format!(
                            "{:>width$}{}",
                            relative_num.abs(),
                            padding,
                            width = line_number_width
                        ),
                        app.config.line_numbers.other_lines_color.to_ratatui_style(),
                    )
                }
            } else {
                let total_width = line_number_width + app.config.line_numbers.padding;
                (" ".repeat(total_width), Style::default())
            }
        } else {
            let total_width = line_number_width + app.config.line_numbers.padding;
            (" ".repeat(total_width), Style::default())
        };

        // Determine base content style - PRIORITY ORDER MATTERS!
        let base_content_style = if is_quick_search_selected {
            // Quick search selected item has highest priority
            app.config.colors.quick_search_selected.to_ratatui_style()
        } else if is_selected {
            match tree_line.line_type {
                LineType::Session => app.config.colors.session_selected.to_ratatui_style(),
                LineType::Window => app.config.colors.window_selected.to_ratatui_style(),
            }
        } else if is_quick_search_match {
            // Dimmed search match (lower priority than selected)
            app.config.colors.quick_search_match.to_ratatui_style()
        } else {
            match tree_line.line_type {
                LineType::Session => app.config.colors.session.to_ratatui_style(),
                LineType::Window => {
                    if tree_line.window.as_ref().is_some_and(|w| w.active) {
                        app.config.colors.window_active.to_ratatui_style()
                    } else {
                        app.config.colors.window_inactive.to_ratatui_style()
                    }
                }
            }
        };

        // Create content spans with highlighting for quick search matches
        let content_spans = if is_quick_search_match
            && !is_quick_search_selected
            && !app.quick_search_query.is_empty()
        {
            // Only do fuzzy highlighting for matches that are NOT the selected item
            // (selected item gets full highlight via base_content_style)

            // Get the search text for this line
            let search_text = match tree_line.line_type {
                LineType::Session => {
                    if let Some(ref session_name) = tree_line.session_name {
                        session_name.clone()
                    } else {
                        tree_line.content.clone()
                    }
                }
                LineType::Window => {
                    if let Some(window) = &tree_line.window {
                        if let Some(ref session_name) = tree_line.session_name {
                            format!("{}:{}", session_name, window.name)
                        } else {
                            window.name.clone()
                        }
                    } else {
                        tree_line.content.clone()
                    }
                }
            };

            // Get match indices for highlighting
            if let Some((_, indices)) = matcher.fuzzy_indices(&search_text, &app.quick_search_query)
            {
                // Create highlighted spans for the display content
                create_highlighted_spans_for_content(
                    &tree_line.content,
                    &search_text,
                    &indices,
                    base_content_style,
                    app.config.colors.search_highlight.to_ratatui_style(),
                )
            } else {
                vec![Span::styled(tree_line.content.clone(), base_content_style)]
            }
        } else {
            // For selected items or non-matches, use the base style (which already has correct priority)
            vec![Span::styled(tree_line.content.clone(), base_content_style)]
        };

        let mut all_spans = vec![Span::styled(line_number_str.clone(), line_number_style)];
        all_spans.extend(content_spans);

        items.push(ListItem::new(Line::from(all_spans)));
    }

    let list = List::new(items);

    f.render_widget(list, area);
}

// Helper function to create highlighted spans for content based on search matches
fn create_highlighted_spans_for_content<'a>(
    display_content: &'a str,
    search_text: &str,
    match_indices: &[usize],
    normal_style: Style,
    highlight_style: Style,
) -> Vec<Span<'a>> {
    // For session:window format, we need to map indices back to display content
    if search_text.contains(':') && !display_content.contains(':') {
        // This is a window where we searched "session:window" but display is just the window content
        // For now, just return the content with normal style since mapping is complex
        vec![Span::styled(display_content.to_string(), normal_style)]
    } else {
        // Direct match - create highlighted spans
        create_highlighted_spans(
            display_content,
            match_indices,
            normal_style,
            highlight_style,
        )
    }
}

fn draw_search_status_bar(f: &mut Frame, app: &App, area: Rect) {
    // Use the same layout as the normal status bar
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14), // Mode indicator
            Constraint::Min(10),    // Help text
            Constraint::Length(30), // Stats/error
        ])
        .split(area);

    // Mode indicator (left)
    let mode_text = get_mode_text(&app.mode);
    let mode_style = get_mode_style(&app.mode, &app.config.colors);
    let mode_paragraph = Paragraph::new(format!(" {mode_text}")).style(mode_style);

    f.render_widget(mode_paragraph, status_chunks[0]);

    // Help text (center)
    render_help_panel(app, status_chunks[1], f.buffer_mut());

    // Right section - show search count
    let right_content = Paragraph::new(format!(" Results: {} ", app.search_results.len()))
        .style(app.config.colors.status_text.to_ratatui_style());

    f.render_widget(right_content, status_chunks[2]);
}
