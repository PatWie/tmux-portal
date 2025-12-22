use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

use crate::config::{Config, get_history_path, load_config};
use crate::search::{SearchPattern, SearchProvider, SearchResult};
use crate::tmux::{
    TmuxSession, TmuxWindow, delete_window, get_current_session_name, get_tmux_sessions,
    kill_session, rename_session, rename_window, switch_to_session, switch_to_window,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Window,
    Rename,
    Search,        // Project search mode (F key) - directory scanning
    QuickSearch,   // Quick search mode (/ key) - search active sessions/windows
    Session,       // Session management mode (S key) - move/reorder sessions
    DeleteConfirm, // Delete confirmation mode (x key) - confirm window deletion
}

#[derive(Debug, Clone)]
pub struct TreeLine {
    pub line_type: LineType,
    pub content: String,
    pub session_name: Option<String>,
    pub window: Option<TmuxWindow>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineType {
    Session,
    Window,
}

pub struct App {
    pub mode: Mode,
    pub previous_mode: Mode,
    pub sessions: Vec<TmuxSession>,
    pub tree_lines: Vec<TreeLine>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub error_message: Option<String>,
    pub show_popup: bool,
    pub popup_input: String,
    pub config: Config,
    pub auto_position_on_active: bool, // Flag to control auto-positioning
    pub search_provider: SearchProvider,
    pub search_results: Vec<SearchResult>,
    pub search_query: String,
    pub search_selected_index: usize,
    // Quick search fields (for / key)
    pub quick_search_query: String,
    pub quick_search_results: Vec<usize>, // Indices into tree_lines that match
    pub quick_search_selected_index: usize,
    // History tracking for digit shortcuts
    pub history: Vec<(String, String)>, // (session_name, window_id)
}

impl App {
    pub fn new() -> Result<Self> {
        let config = load_config()?;

        // Create search patterns from config
        let mut search_patterns = Vec::new();

        // Add patterns from new config format
        for pattern_config in &config.search_patterns {
            fn fun_name(p: &String) -> std::path::PathBuf {
                std::path::PathBuf::from(p)
            }
            let paths: Vec<std::path::PathBuf> =
                pattern_config.paths.iter().map(fun_name).collect();

            search_patterns.push(SearchPattern::new(
                pattern_config.name.clone(),
                paths,
                pattern_config.pattern.clone(),
            ));
        }

        // Legacy support: convert old search_paths to git-style pattern
        if !config.search_paths.is_empty() && search_patterns.is_empty() {
            fn fun_name(p: &String) -> std::path::PathBuf {
                std::path::PathBuf::from(p)
            }
            let paths: Vec<std::path::PathBuf> = config.search_paths.iter().map(fun_name).collect();

            search_patterns.push(SearchPattern::new(
                "git-style".to_string(),
                paths,
                "{session}/{window}".to_string(),
            ));
        }

        let mut search_provider = SearchProvider::new(search_patterns);

        // Scan directories on startup (in background, don't fail if it errors)
        let _ = search_provider.scan_directories();

        let mut app = Self {
            mode: Mode::Window,
            previous_mode: Mode::Window,
            sessions: Vec::new(),
            tree_lines: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            error_message: None,
            show_popup: false,
            popup_input: String::new(),
            config,
            auto_position_on_active: true, // Enable auto-positioning on startup
            search_provider,
            search_results: Vec::new(),
            search_query: String::new(),
            search_selected_index: 0,
            quick_search_query: String::new(),
            quick_search_results: Vec::new(),
            quick_search_selected_index: 0,
            history: Self::load_history().unwrap_or_default(),
        };

        app.refresh_sessions()?;
        Ok(app)
    }

    pub fn refresh_sessions(&mut self) -> Result<()> {
        self.sessions = get_tmux_sessions()?;
        self.rebuild_tree_view();

        // Only auto-position on active window if the flag is set
        if self.auto_position_on_active {
            self.position_on_active_window();
            self.auto_position_on_active = false; // Disable after first use
        }

        self.ensure_valid_selection();
        Ok(())
    }

    fn rebuild_tree_view(&mut self) {
        self.tree_lines.clear();

        if self.sessions.is_empty() {
            return;
        }

        // Build individual session trees (each session is a root node)
        let mut all_tree_lines = Vec::new();

        for session in &self.sessions {
            let mut window_nodes = Vec::new();

            // Check for duplicate window names in this session (only if config enabled)
            let show_ids = if self.config.show_window_ids {
                let mut name_counts = std::collections::HashMap::new();
                for window in &session.windows {
                    *name_counts.entry(&window.name).or_insert(0) += 1;
                }
                name_counts.values().any(|&count| count > 1)
            } else {
                false
            };

            for (window_idx, window) in session.windows.iter().enumerate() {
                let window_display = if show_ids {
                    // Show ID for disambiguation when there are duplicates
                    if window.active {
                        format!("{} [{}] (active)", window.name, window.id)
                    } else {
                        format!("{} [{}]", window.name, window.id)
                    }
                } else {
                    // Show normally when no duplicates or config disabled
                    if window.active {
                        format!("{} (active)", window.name)
                    } else {
                        window.name.clone()
                    }
                };
                window_nodes.push((window_idx, StringTreeNode::new(window_display)));
            }

            let session_tree = if window_nodes.is_empty() {
                StringTreeNode::new(session.name.clone())
            } else {
                StringTreeNode::with_child_nodes(
                    session.name.clone(),
                    window_nodes.iter().map(|(_, node)| node.clone()),
                )
            };

            // Use box drawing characters
            let formatting = TreeFormatting::dir_tree(FormatCharacters::box_chars());
            let tree_output = session_tree
                .to_string_with_format(&formatting)
                .unwrap_or_else(|_| session_tree.to_string());

            // Parse this session's tree output
            for (line_idx, line) in tree_output.lines().enumerate() {
                if line_idx == 0 {
                    // This is the session line (root of this tree)
                    all_tree_lines.push(TreeLine {
                        line_type: LineType::Session,
                        content: line.to_string(),
                        session_name: Some(session.name.clone()),
                        window: None,
                    });
                } else {
                    // This is a window line - use the window index to get the correct window
                    let window_idx = line_idx - 1; // Subtract 1 because line 0 is the session
                    if window_idx < session.windows.len() {
                        let window = &session.windows[window_idx];
                        all_tree_lines.push(TreeLine {
                            line_type: LineType::Window,
                            content: line.to_string(),
                            session_name: Some(window.session_name.clone()),
                            window: Some(window.clone()),
                        });
                    }
                }
            }
        }

        self.tree_lines = all_tree_lines;
    }

    fn position_on_active_window(&mut self) {
        // Get the current session name from tmux
        let current_session = match get_current_session_name() {
            Ok(Some(session_name)) => session_name,
            _ => {
                // Fallback: find any active window if we can't detect current session
                for (index, line) in self.tree_lines.iter().enumerate() {
                    if line.line_type == LineType::Window {
                        if let Some(window) = &line.window {
                            if window.active {
                                self.selected_index = index;
                                return;
                            }
                        }
                    }
                }
                return;
            }
        };

        // Find the active window within the current session
        for (index, line) in self.tree_lines.iter().enumerate() {
            if line.line_type == LineType::Window {
                if let (Some(line_session), Some(window)) = (&line.session_name, &line.window) {
                    if line_session == &current_session && window.active {
                        self.selected_index = index;
                        return;
                    }
                }
            }
        }

        // If no active window found in current session, position on the session itself
        for (index, line) in self.tree_lines.iter().enumerate() {
            if line.line_type == LineType::Session {
                if let Some(line_session) = &line.session_name {
                    if line_session == &current_session {
                        self.selected_index = index;
                        return;
                    }
                }
            }
        }
    }

    pub fn ensure_valid_selection(&mut self) {
        if self.tree_lines.is_empty() {
            self.selected_index = 0;
            return;
        }

        // Find the first window line at or after current selection
        for i in self.selected_index..self.tree_lines.len() {
            if self.tree_lines[i].line_type == LineType::Window {
                self.selected_index = i;
                return;
            }
        }

        // If no window found after current selection, search from beginning
        for i in 0..self.selected_index {
            if self.tree_lines[i].line_type == LineType::Window {
                self.selected_index = i;
                return;
            }
        }

        // If no windows at all, stay at 0
        self.selected_index = 0;
    }

    pub fn update_scroll_offset(&mut self, viewport_height: usize) {
        if self.tree_lines.is_empty() || viewport_height == 0 {
            self.scroll_offset = 0;
            return;
        }

        let viewport_height = viewport_height.saturating_sub(1); // Account for borders/padding
        
        // If selected item is above the current viewport, scroll up
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        // If selected item is below the current viewport, scroll down
        else if self.selected_index >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.selected_index.saturating_sub(viewport_height.saturating_sub(1));
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        self.error_message = None;

        match self.mode {
            Mode::Window => self.handle_normal_mode(key),
            Mode::Rename => self.handle_insert_mode(key),
            Mode::Search => self.handle_search_input_mode(key),
            Mode::QuickSearch => self.handle_quick_search_mode(key),
            Mode::Session => self.handle_session_mode(key),
            Mode::DeleteConfirm => self.handle_delete_confirm_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => return Ok(true), // Quit the app
            KeyCode::Char('q') => return Ok(true),
            // Handle Shift+Arrow keys first (for window reordering)
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => self.move_item_up()?,
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.move_item_down()?
            }
            // Then handle regular navigation
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.move_to_top();
                }
            }
            KeyCode::Char('G') => self.move_to_bottom(),
            KeyCode::Enter => {
                if self.activate_selected()? {
                    return Ok(true);
                }
            }
            KeyCode::Char('r') | KeyCode::Char(',') => self.start_rename(),
            KeyCode::Char('x') => self.start_delete_confirm(),
            KeyCode::Char('R') => {
                self.auto_position_on_active = true; // Re-enable auto-positioning for manual refresh
                self.refresh_sessions()?
            }
            KeyCode::Char('/') => self.start_quick_search(),
            KeyCode::Char('F') => self.start_project_search(),
            KeyCode::Char('S') => self.start_session_mode(),
            KeyCode::Char('J') => self.move_item_down()?,
            KeyCode::Char('K') => self.move_item_up()?,
            KeyCode::Char('C') => self.create_new_window()?,
            // Digit shortcuts for history navigation
            KeyCode::Char('1') => return self.jump_to_history(0),
            KeyCode::Char('2') => return self.jump_to_history(1),
            KeyCode::Char('3') => return self.jump_to_history(2),
            KeyCode::Char('4') => return self.jump_to_history(3),
            KeyCode::Char('5') => return self.jump_to_history(4),
            KeyCode::Char('6') => return self.jump_to_history(5),
            KeyCode::Char('7') => return self.jump_to_history(6),
            KeyCode::Char('8') => return self.jump_to_history(7),
            KeyCode::Char('9') => return self.jump_to_history(8),
            KeyCode::Char('0') => return self.jump_to_history(9),
            _ => {}
        }

        Ok(false)
    }

    fn handle_insert_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Window;
                self.show_popup = false;
                self.popup_input.clear();
            }
            KeyCode::Enter => {
                self.confirm_rename()?;
            }
            KeyCode::Backspace => {
                self.popup_input.pop();
            }
            KeyCode::Char(c) => {
                self.popup_input.push(c);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_search_input_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Window;
                Ok(false)
            }
            KeyCode::Enter => {
                if !self.search_results.is_empty() {
                    // execute_search_selection returns true if we should exit
                    self.execute_search_selection()
                } else {
                    self.mode = Mode::Window;
                    Ok(false)
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_search_results();
                Ok(false)
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search_results();
                Ok(false)
            }
            KeyCode::Up => {
                if self.search_selected_index > 0 {
                    self.search_selected_index -= 1;
                }
                Ok(false)
            }
            KeyCode::Down => {
                if self.search_selected_index < self.search_results.len().saturating_sub(1) {
                    self.search_selected_index += 1;
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn move_item_up(&mut self) -> Result<()> {
        if self.selected_index == 0 || self.tree_lines.is_empty() {
            return Ok(());
        }

        let current_line = &self.tree_lines[self.selected_index];

        // Only allow moving windows, not sessions
        if current_line.line_type != LineType::Window {
            return Ok(());
        }

        // Find the previous window in the same session
        let current_session = current_line.session_name.as_ref().unwrap();
        let mut prev_window_index = None;

        for i in (0..self.selected_index).rev() {
            let line = &self.tree_lines[i];
            if line.line_type == LineType::Window
                && line.session_name.as_ref() == Some(current_session)
            {
                prev_window_index = Some(i);
                break;
            }
        }

        if let Some(prev_idx) = prev_window_index {
            // Get the window IDs and check if we're moving the active window
            let current_window_id = self.tree_lines[self.selected_index]
                .window
                .as_ref()
                .unwrap()
                .id
                .clone();
            let prev_window_id = self.tree_lines[prev_idx]
                .window
                .as_ref()
                .unwrap()
                .id
                .clone();
            let was_current_active = self.tree_lines[self.selected_index]
                .window
                .as_ref()
                .unwrap()
                .active;

            // Perform the swap in tmux immediately
            if let Err(e) =
                self.swap_windows_in_tmux(current_session, &current_window_id, &prev_window_id)
            {
                self.error_message = Some(format!("Failed to swap windows: {e}"));
                return Ok(());
            }

            // Refresh to get the updated state from tmux
            self.refresh_sessions()?;

            // If we moved the active window, find where it ended up and select it
            if was_current_active {
                if let Some(new_index) = self.find_window_index_by_id(&current_window_id) {
                    self.selected_index = new_index;
                }
            } else {
                // If we moved a non-active window, try to maintain selection on it
                if let Some(new_index) = self.find_window_index_by_id(&current_window_id) {
                    self.selected_index = new_index;
                }
            }
        }

        Ok(())
    }

    fn move_item_down(&mut self) -> Result<()> {
        if self.selected_index >= self.tree_lines.len() - 1 {
            return Ok(());
        }

        let current_line = &self.tree_lines[self.selected_index];

        // Only allow moving windows, not sessions
        if current_line.line_type != LineType::Window {
            return Ok(());
        }

        // Find the next window in the same session
        let current_session = current_line.session_name.as_ref().unwrap();
        let mut next_window_index = None;

        for i in (self.selected_index + 1)..self.tree_lines.len() {
            let line = &self.tree_lines[i];
            if line.line_type == LineType::Window
                && line.session_name.as_ref() == Some(current_session)
            {
                next_window_index = Some(i);
                break;
            }
        }

        if let Some(next_idx) = next_window_index {
            // Get the window IDs and check if we're moving the active window
            let current_window_id = self.tree_lines[self.selected_index]
                .window
                .as_ref()
                .unwrap()
                .id
                .clone();
            let next_window_id = self.tree_lines[next_idx]
                .window
                .as_ref()
                .unwrap()
                .id
                .clone();
            let was_current_active = self.tree_lines[self.selected_index]
                .window
                .as_ref()
                .unwrap()
                .active;

            // Perform the swap in tmux immediately
            if let Err(e) =
                self.swap_windows_in_tmux(current_session, &current_window_id, &next_window_id)
            {
                self.error_message = Some(format!("Failed to swap windows: {e}"));
                return Ok(());
            }

            // Refresh to get the updated state from tmux
            self.refresh_sessions()?;

            // If we moved the active window, find where it ended up and select it
            if was_current_active {
                if let Some(new_index) = self.find_window_index_by_id(&current_window_id) {
                    self.selected_index = new_index;
                }
            } else {
                // If we moved a non-active window, try to maintain selection on it
                if let Some(new_index) = self.find_window_index_by_id(&current_window_id) {
                    self.selected_index = new_index;
                }
            }
        }

        Ok(())
    }

    fn find_window_index_by_id(&self, window_id: &str) -> Option<usize> {
        self.tree_lines.iter().position(|line| {
            line.line_type == LineType::Window
                && line.window.as_ref().is_some_and(|w| w.id == window_id)
        })
    }

    // Add a method to handle individual window swaps during J/K operations
    fn swap_windows_in_tmux(
        &self,
        session_name: &str,
        window1_id: &str,
        window2_id: &str,
    ) -> Result<()> {
        crate::tmux::swap_windows_in_tmux(session_name, window1_id, window2_id)
    }

    fn move_down(&mut self) {
        if self.tree_lines.is_empty() {
            return;
        }

        let mut next_index = self.selected_index;
        for i in (self.selected_index + 1)..self.tree_lines.len() {
            if self.tree_lines[i].line_type == LineType::Window {
                next_index = i;
                break;
            }
        }
        self.selected_index = next_index;
    }

    fn move_up(&mut self) {
        if self.tree_lines.is_empty() || self.selected_index == 0 {
            return;
        }

        let mut prev_index = self.selected_index;
        for i in (0..self.selected_index).rev() {
            if self.tree_lines[i].line_type == LineType::Window {
                prev_index = i;
                break;
            }
        }
        self.selected_index = prev_index;
    }

    fn move_to_top(&mut self) {
        for i in 0..self.tree_lines.len() {
            if self.tree_lines[i].line_type == LineType::Window {
                self.selected_index = i;
                break;
            }
        }
    }

    fn move_to_bottom(&mut self) {
        for i in (0..self.tree_lines.len()).rev() {
            if self.tree_lines[i].line_type == LineType::Window {
                self.selected_index = i;
                break;
            }
        }
    }

    fn activate_selected(&mut self) -> Result<bool> {
        if let Some(line) = self.tree_lines.get(self.selected_index) {
            if let Some(window) = &line.window {
                let session_name = window.session_name.clone();
                let window_id = window.id.clone();
                
                // Add to history before switching
                self.add_to_history(&session_name, &window_id);
                
                match switch_to_window(&session_name, &window_id) {
                    Ok(_) => return Ok(true), // Exit the app after successful switch
                    Err(e) => {
                        self.error_message = Some(format!("Failed to switch: {e}"));
                    }
                }
            }
        }
        Ok(false)
    }

    fn start_rename(&mut self) {
        if let Some(line) = self.tree_lines.get(self.selected_index) {
            match line.line_type {
                LineType::Window => {
                    if let Some(window) = &line.window {
                        self.previous_mode = self.mode.clone();
                        self.mode = Mode::Rename;
                        self.show_popup = true;
                        self.popup_input = window.name.clone();
                    }
                }
                LineType::Session => {
                    if let Some(session_name) = &line.session_name {
                        self.previous_mode = self.mode.clone();
                        self.mode = Mode::Rename;
                        self.show_popup = true;
                        self.popup_input = session_name.clone();
                    }
                }
            }
        }
    }

    fn start_delete_confirm(&mut self) {
        if let Some(line) = self.tree_lines.get(self.selected_index) {
            if let Some(window) = &line.window {
                self.previous_mode = self.mode.clone();
                self.mode = Mode::DeleteConfirm;
                self.show_popup = true;
                self.popup_input = format!("Delete window '{}'? (y/N)", window.name);
            }
        }
    }

    fn confirm_rename(&mut self) -> Result<()> {
        let was_session_mode = self.previous_mode == Mode::Session;

        if let Some(line) = self.tree_lines.get(self.selected_index) {
            match line.line_type {
                LineType::Window => {
                    if let Some(window) = &line.window {
                        match rename_window(&window.session_name, &window.id, &self.popup_input) {
                            Ok(_) => {
                                self.refresh_sessions()?;
                                self.rebuild_tree_view();
                                if was_session_mode {
                                    // In session mode, ensure we're positioned on a session
                                    self.move_to_first_session();
                                }
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to rename window: {e}"));
                            }
                        }
                    }
                }
                LineType::Session => {
                    if let Some(session_name) = &line.session_name {
                        match rename_session(session_name, &self.popup_input) {
                            Ok(_) => {
                                self.refresh_sessions()?;
                                self.rebuild_tree_view();
                                if was_session_mode {
                                    // Find the renamed session and position on it
                                    for (index, line) in self.tree_lines.iter().enumerate() {
                                        if line.line_type == LineType::Session
                                            && line.session_name.as_ref() == Some(&self.popup_input)
                                        {
                                            self.selected_index = index;
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to rename session: {e}"));
                            }
                        }
                    }
                }
            }
        }

        // Return to the previous mode
        self.mode = self.previous_mode.clone();
        self.show_popup = false;
        self.popup_input.clear();
        Ok(())
    }

    fn confirm_delete(&mut self) -> Result<()> {
        let was_session_mode = self.previous_mode == Mode::Session;

        if let Some(line) = self.tree_lines.get(self.selected_index) {
            match line.line_type {
                LineType::Window => {
                    if let Some(window) = &line.window {
                        match delete_window(&window.session_name, &window.id) {
                            Ok(_) => {
                                self.refresh_sessions()?;
                                self.rebuild_tree_view();
                                if was_session_mode {
                                    // In session mode, ensure we're positioned on a session
                                    self.move_to_first_session();
                                } else {
                                    // If we deleted the currently selected window, move selection to a safe position
                                    if self.selected_index >= self.tree_lines.len()
                                        && self.selected_index > 0
                                    {
                                        self.selected_index = self.tree_lines.len() - 1;
                                    }
                                }
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to delete window: {e}"));
                            }
                        }
                    }
                }
                LineType::Session => {
                    if let Some(session_name) = &line.session_name {
                        match kill_session(session_name) {
                            Ok(_) => {
                                self.refresh_sessions()?;
                                self.rebuild_tree_view();
                                if was_session_mode {
                                    // Position on the first available session
                                    self.move_to_first_session();
                                } else {
                                    self.ensure_valid_selection();
                                }
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to delete session: {e}"));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn start_project_search(&mut self) {
        self.mode = Mode::Search;
        self.search_query.clear();
        self.search_selected_index = 0;
        // Perform initial search with empty query (shows all results)
        self.update_search_results();
    }

    fn start_session_mode(&mut self) {
        self.mode = Mode::Session;
        // In session mode, we show the full tree but navigate between sessions only
        self.rebuild_tree_view();
        // Position on the first session
        self.move_to_first_session();
    }

    fn move_to_first_session(&mut self) {
        for (index, line) in self.tree_lines.iter().enumerate() {
            if line.line_type == LineType::Session {
                self.selected_index = index;
                break;
            }
        }
    }

    fn start_quick_search(&mut self) {
        self.mode = Mode::QuickSearch;
        self.quick_search_query.clear();
        self.quick_search_selected_index = 0;
        // Perform initial search with empty query (shows all active sessions/windows)
        self.update_quick_search_results();
    }

    fn update_search_results(&mut self) {
        self.search_results = self.search_provider.search(&self.search_query);
        self.search_selected_index = 0; // Reset selection when results change
    }

    fn update_quick_search_results(&mut self) {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;

        let matcher = SkimMatcherV2::default().ignore_case();

        if self.quick_search_query.is_empty() {
            // Show all sessions and windows
            self.quick_search_results = (0..self.tree_lines.len()).collect();
        } else {
            // Fuzzy search through session:window format and sort by score
            let mut scored_results: Vec<(usize, i64)> = self
                .tree_lines
                .iter()
                .enumerate()
                .filter_map(|(i, line)| {
                    let search_text = match line.line_type {
                        LineType::Session => {
                            // For sessions, just search the session name
                            if let Some(ref session_name) = line.session_name {
                                session_name.clone()
                            } else {
                                return None;
                            }
                        }
                        LineType::Window => {
                            // For windows, search in session:window format
                            if let Some(window) = &line.window {
                                if let Some(ref session_name) = line.session_name {
                                    format!("{}:{}", session_name, window.name)
                                } else {
                                    window.name.clone()
                                }
                            } else {
                                return None;
                            }
                        }
                    };

                    // Get the fuzzy match score
                    matcher
                        .fuzzy_match(&search_text, &self.quick_search_query)
                        .map(|score| (i, score))
                })
                .collect();

            // Sort by score (higher scores are better matches)
            scored_results.sort_by(|a, b| b.1.cmp(&a.1));

            // Extract just the indices
            self.quick_search_results = scored_results.into_iter().map(|(i, _)| i).collect();
        }

        self.quick_search_selected_index = 0; // Reset selection when results change
    }

    fn handle_quick_search_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Window;
                Ok(false)
            }
            KeyCode::Enter => {
                if !self.quick_search_results.is_empty() {
                    // Jump to the selected line in the tree
                    let selected_tree_index =
                        self.quick_search_results[self.quick_search_selected_index];
                    self.selected_index = selected_tree_index;
                    self.mode = Mode::Window;

                    // Activate the selected item (switch to session/window)
                    if self.activate_selected()? {
                        return Ok(true); // Exit if activation was successful
                    }
                } else {
                    self.mode = Mode::Window;
                }
                Ok(false)
            }
            KeyCode::Char(c) => {
                self.quick_search_query.push(c);
                self.update_quick_search_results();
                Ok(false)
            }
            KeyCode::Backspace => {
                self.quick_search_query.pop();
                self.update_quick_search_results();
                Ok(false)
            }
            KeyCode::Up => {
                if self.quick_search_selected_index > 0 {
                    self.quick_search_selected_index -= 1;
                }
                Ok(false)
            }
            KeyCode::Down => {
                if self.quick_search_selected_index
                    < self.quick_search_results.len().saturating_sub(1)
                {
                    self.quick_search_selected_index += 1;
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn execute_search_selection(&mut self) -> Result<bool> {
        if self.search_selected_index < self.search_results.len() {
            let selected = &self.search_results[self.search_selected_index];

            // Use the same logic as the bash script
            self.switch_to_session_and_window(
                &selected.session_name,
                &selected.window_name,
                &selected.full_path,
            )?;

            // Return true to indicate the application should exit
            return Ok(true);
        }
        Ok(false)
    }

    fn switch_to_session_and_window(
        &self,
        session_name: &str,
        window_name: &str,
        path: &std::path::Path,
    ) -> Result<()> {
        crate::tmux::switch_to_session_and_window(session_name, window_name, path)
    }

    pub fn get_window_line_numbers(&self) -> HashMap<usize, i32> {
        let mut line_numbers = HashMap::new();
        let window_indices: Vec<usize> = self
            .tree_lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| {
                if line.line_type == LineType::Window {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        if let Some(selected_pos) = window_indices
            .iter()
            .position(|&i| i == self.selected_index)
        {
            for (pos, &line_idx) in window_indices.iter().enumerate() {
                let relative_num = pos as i32 - selected_pos as i32;
                line_numbers.insert(line_idx, relative_num);
            }
        }

        line_numbers
    }

    fn handle_session_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => {
                self.mode = Mode::Window;
                // Tree view is already built, just ensure valid selection for normal mode
                self.ensure_valid_selection();
            }
            KeyCode::Esc => {
                self.mode = Mode::Window;
                // Tree view is already built, just ensure valid selection for normal mode
                self.ensure_valid_selection();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down_session_mode();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up_session_mode();
            }
            KeyCode::Char('g') => {
                self.move_to_top_session_mode();
            }
            KeyCode::Char('G') => {
                self.move_to_bottom_session_mode();
            }
            KeyCode::Char('J') => {
                // Move session down in order
                self.move_session_down()?;
            }
            KeyCode::Char('K') => {
                // Move session up in order
                self.move_session_up()?;
            }
            KeyCode::Enter => {
                // Switch to selected session
                if self.activate_selected_session()? {
                    return Ok(true); // Exit the app after successful switch
                }
            }
            KeyCode::Char('r') | KeyCode::Char(',') => {
                // Rename session
                self.start_rename();
            }
            KeyCode::Char('x') => {
                // Delete session (with confirmation)
                self.start_delete_session_confirm();
            }
            KeyCode::Char('R') => {
                // Refresh sessions
                self.refresh_sessions()?;
                self.rebuild_tree_view();
                self.move_to_first_session();
            }
            _ => {}
        }
        Ok(false)
    }

    fn move_down_session_mode(&mut self) {
        if self.tree_lines.is_empty() {
            return;
        }

        // Find the next session after the current selection
        for i in (self.selected_index + 1)..self.tree_lines.len() {
            if self.tree_lines[i].line_type == LineType::Session {
                self.selected_index = i;
                return;
            }
        }
        // If no session found after current position, stay at current position
    }

    fn move_up_session_mode(&mut self) {
        if self.tree_lines.is_empty() {
            return;
        }

        // Find the previous session before the current selection
        for i in (0..self.selected_index).rev() {
            if self.tree_lines[i].line_type == LineType::Session {
                self.selected_index = i;
                return;
            }
        }
        // If no session found before current position, stay at current position
    }

    fn move_to_top_session_mode(&mut self) {
        // Find the first session
        for (index, line) in self.tree_lines.iter().enumerate() {
            if line.line_type == LineType::Session {
                self.selected_index = index;
                break;
            }
        }
    }

    fn move_to_bottom_session_mode(&mut self) {
        // Find the last session
        for (index, line) in self.tree_lines.iter().enumerate().rev() {
            if line.line_type == LineType::Session {
                self.selected_index = index;
                break;
            }
        }
    }

    fn activate_selected_session(&mut self) -> Result<bool> {
        if let Some(line) = self.tree_lines.get(self.selected_index) {
            if let Some(session_name) = &line.session_name {
                match switch_to_session(session_name) {
                    Ok(_) => return Ok(true), // Exit the app after successful switch
                    Err(e) => {
                        self.error_message = Some(format!("Failed to switch to session: {e}"));
                    }
                }
            }
        }
        Ok(false)
    }

    fn start_delete_session_confirm(&mut self) {
        if let Some(line) = self.tree_lines.get(self.selected_index) {
            if let Some(session_name) = &line.session_name {
                self.previous_mode = self.mode.clone();
                self.mode = Mode::DeleteConfirm;
                self.show_popup = true;
                self.popup_input = format!("Delete session '{session_name}'? (y/N)");
            }
        }
    }

    fn move_session_up(&mut self) -> Result<()> {
        // Ensure we're on a session line
        if let Some(current_line) = self.tree_lines.get(self.selected_index) {
            if current_line.line_type != LineType::Session {
                return Ok(());
            }
        } else {
            return Ok(());
        }

        // Find the previous session
        let mut prev_session_index = None;
        for i in (0..self.selected_index).rev() {
            if self.tree_lines[i].line_type == LineType::Session {
                prev_session_index = Some(i);
                break;
            }
        }

        if let Some(prev_idx) = prev_session_index {
            // Get session names
            let current_session = self.tree_lines[self.selected_index]
                .session_name
                .as_ref()
                .unwrap()
                .clone();
            let prev_session = self.tree_lines[prev_idx]
                .session_name
                .as_ref()
                .unwrap()
                .clone();

            // Swap sessions in our local list
            let current_idx = self
                .sessions
                .iter()
                .position(|s| s.name == current_session)
                .unwrap();
            let prev_session_idx = self
                .sessions
                .iter()
                .position(|s| s.name == prev_session)
                .unwrap();

            self.sessions.swap(current_idx, prev_session_idx);

            // Rebuild tree view and position on the moved session
            self.rebuild_tree_view();

            // Find the session that was moved and position on it
            for (index, line) in self.tree_lines.iter().enumerate() {
                if line.line_type == LineType::Session
                    && line.session_name.as_ref() == Some(&current_session)
                {
                    self.selected_index = index;
                    break;
                }
            }
        }

        Ok(())
    }

    fn move_session_down(&mut self) -> Result<()> {
        // Ensure we're on a session line
        if let Some(current_line) = self.tree_lines.get(self.selected_index) {
            if current_line.line_type != LineType::Session {
                return Ok(());
            }
        } else {
            return Ok(());
        }

        // Find the next session
        let mut next_session_index = None;
        for i in (self.selected_index + 1)..self.tree_lines.len() {
            if self.tree_lines[i].line_type == LineType::Session {
                next_session_index = Some(i);
                break;
            }
        }

        if let Some(next_idx) = next_session_index {
            // Get session names
            let current_session = self.tree_lines[self.selected_index]
                .session_name
                .as_ref()
                .unwrap()
                .clone();
            let next_session = self.tree_lines[next_idx]
                .session_name
                .as_ref()
                .unwrap()
                .clone();

            // Swap sessions in our local list
            let current_idx = self
                .sessions
                .iter()
                .position(|s| s.name == current_session)
                .unwrap();
            let next_session_idx = self
                .sessions
                .iter()
                .position(|s| s.name == next_session)
                .unwrap();

            self.sessions.swap(current_idx, next_session_idx);

            // Rebuild tree view and position on the moved session
            self.rebuild_tree_view();

            // Find the session that was moved and position on it
            for (index, line) in self.tree_lines.iter().enumerate() {
                if line.line_type == LineType::Session
                    && line.session_name.as_ref() == Some(&current_session)
                {
                    self.selected_index = index;
                    break;
                }
            }
        }

        Ok(())
    }

    fn create_new_window(&mut self) -> Result<()> {
        // Get the current session name
        let current_session = match get_current_session_name() {
            Ok(Some(session_name)) => session_name,
            _ => {
                // If we can't get the current session, check if there's a selected session
                if let Some(line) = self.tree_lines.get(self.selected_index) {
                    if let Some(session_name) = &line.session_name {
                        session_name.clone()
                    } else {
                        self.error_message = Some("No session selected".to_string());
                        return Ok(());
                    }
                } else {
                    self.error_message = Some("No session selected".to_string());
                    return Ok(());
                }
            }
        };

        // Create a new window in the session
        if let Err(e) = crate::tmux::create_new_window(&current_session) {
            self.error_message = Some(format!("Failed to create new window: {e}"));
            return Ok(());
        }

        // Refresh sessions to get the new window
        self.refresh_sessions()?;

        // Find the newly created window (should be the last window in the session)
        // and position the cursor on it
        let mut last_window_index = None;
        for (i, line) in self.tree_lines.iter().enumerate().rev() {
            if line.line_type == LineType::Window
                && line.session_name.as_ref() == Some(&current_session)
            {
                last_window_index = Some(i);
                break;
            }
        }

        if let Some(index) = last_window_index {
            self.selected_index = index;
        }

        Ok(())
    }

    fn add_to_history(&mut self, session_name: &str, window_id: &str) {
        let entry = (session_name.to_string(), window_id.to_string());
        
        // Remove if already exists
        self.history.retain(|h| h != &entry);
        
        // Add to front
        self.history.insert(0, entry);
        
        // Keep only last 10
        self.history.truncate(10);
        
        // Save to disk
        let _ = Self::save_history(&self.history);
    }

    fn load_history() -> Result<Vec<(String, String)>> {
        let path = get_history_path()?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(path)?;
        let history = serde_json::from_str(&content)?;
        Ok(history)
    }

    fn save_history(history: &[(String, String)]) -> Result<()> {
        let path = get_history_path()?;
        let content = serde_json::to_string(history)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn jump_to_history(&mut self, index: usize) -> Result<bool> {
        if let Some((session_name, window_id)) = self.history.get(index).cloned() {
            match switch_to_window(&session_name, &window_id) {
                Ok(_) => return Ok(true),
                Err(e) => {
                    self.error_message = Some(format!("Failed to switch: {e}"));
                }
            }
        }
        Ok(false)
    }

    fn handle_delete_confirm_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = self.previous_mode.clone();
                self.show_popup = false;
                self.popup_input.clear();
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Confirm deletion
                if let Err(e) = self.confirm_delete() {
                    self.error_message = Some(format!("Failed to delete: {e}"));
                }
                self.mode = self.previous_mode.clone();
                self.show_popup = false;
                self.popup_input.clear();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                // Cancel deletion
                self.mode = self.previous_mode.clone();
                self.show_popup = false;
                self.popup_input.clear();
            }
            _ => {}
        }
        Ok(false)
    }
}
