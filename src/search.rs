use anyhow::Result;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub display_text: String,
    pub session_name: String,
    pub window_name: String,
    pub full_path: PathBuf,
    pub score: i64,
    pub match_indices: Vec<usize>, // Indices of characters that matched the query
}

#[derive(Debug, Clone)]
pub struct SearchPattern {
    pub name: String,
    pub base_paths: Vec<PathBuf>,
    pub pattern: String, // e.g., "{session}/{window}" or "{session}/src/{window}"
}

impl SearchPattern {
    pub fn new(name: String, base_paths: Vec<PathBuf>, pattern: String) -> Self {
        Self {
            name,
            base_paths,
            pattern,
        }
    }

    // Parse pattern like "{session}/src/{window}" into components
    fn parse_pattern(&self) -> Vec<PatternComponent> {
        let mut components = Vec::new();
        let parts: Vec<&str> = self.pattern.split('/').collect();

        // Check if pattern contains a session placeholder
        let has_session_placeholder = parts.iter().any(|part| {
            part.starts_with('{') && part.ends_with('}') && &part[1..part.len() - 1] == "session"
        });

        // If no session placeholder, use the pattern name as the fixed session name
        if !has_session_placeholder {
            components.push(PatternComponent::FixedSession(self.name.clone()));
        }

        for part in parts {
            if part.starts_with('{') && part.ends_with('}') {
                let var_name = &part[1..part.len() - 1];
                match var_name {
                    "session" => components.push(PatternComponent::Session),
                    "window" => components.push(PatternComponent::Window),
                    _ => components.push(PatternComponent::Literal(part.to_string())),
                }
            } else {
                components.push(PatternComponent::Literal(part.to_string()));
            }
        }

        components
    }
}

#[derive(Debug, Clone)]
enum PatternComponent {
    Session,
    Window,
    Literal(String),
    FixedSession(String), // New component for fixed session names
}

pub struct SearchProvider {
    matcher: SkimMatcherV2,
    patterns: Vec<SearchPattern>,
    cached_results: Vec<SearchResult>,
}

impl SearchProvider {
    pub fn new(patterns: Vec<SearchPattern>) -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            patterns,
            cached_results: Vec::new(),
        }
    }

    // Legacy constructor for backward compatibility

    pub fn scan_directories(&mut self) -> Result<()> {
        self.cached_results.clear();

        let patterns = self.patterns.clone(); // Clone to avoid borrowing issues
        for pattern in &patterns {
            self.scan_pattern(pattern)?;
        }

        Ok(())
    }

    fn scan_pattern(&mut self, pattern: &SearchPattern) -> Result<()> {
        let components = pattern.parse_pattern();

        for base_path in &pattern.base_paths {
            if !base_path.exists() {
                continue;
            }

            self.scan_with_pattern(base_path, &components, &mut Vec::new())?;
        }

        Ok(())
    }

    fn scan_with_pattern(
        &mut self,
        current_path: &Path,
        remaining_components: &[PatternComponent],
        captured_values: &mut Vec<(PatternComponent, String)>,
    ) -> Result<()> {
        if remaining_components.is_empty() {
            // We've matched the full pattern, extract session and window names
            let mut session_name = String::new();
            let mut window_name = String::new();

            for (component, value) in captured_values {
                match component {
                    PatternComponent::Session => session_name = value.clone(),
                    PatternComponent::Window => window_name = value.clone(),
                    PatternComponent::FixedSession(name) => session_name = name.clone(),
                    _ => {}
                }
            }

            if !session_name.is_empty() && !window_name.is_empty() {
                let display_text = format!("{session_name}/{window_name}");

                self.cached_results.push(SearchResult {
                    display_text,
                    session_name,
                    window_name,
                    full_path: current_path.to_path_buf(),
                    score: 0,
                    match_indices: Vec::new(), // Empty for cached results
                });
            }

            return Ok(());
        }

        let current_component = &remaining_components[0];
        let remaining = &remaining_components[1..];

        match current_component {
            PatternComponent::Literal(literal) => {
                // Must match this literal directory name
                let next_path = current_path.join(literal);
                if next_path.exists() && next_path.is_dir() {
                    self.scan_with_pattern(&next_path, remaining, captured_values)?;
                }
            }
            PatternComponent::Session | PatternComponent::Window => {
                // Scan all subdirectories and capture their names
                if let Ok(entries) = fs::read_dir(current_path) {
                    for entry in entries {
                        let entry = entry?;
                        let entry_path = entry.path();

                        if !entry_path.is_dir() {
                            continue;
                        }

                        if let Some(dir_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                            captured_values.push((current_component.clone(), dir_name.to_string()));
                            self.scan_with_pattern(&entry_path, remaining, captured_values)?;
                            captured_values.pop();
                        }
                    }
                }
            }
            PatternComponent::FixedSession(name) => {
                // Add the fixed session name to captured values and continue
                captured_values.push((current_component.clone(), name.clone()));
                self.scan_with_pattern(current_path, remaining, captured_values)?;
                captured_values.pop();
            }
        }

        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return self.cached_results.clone();
        }

        let mut results: Vec<SearchResult> = self
            .cached_results
            .iter()
            .filter_map(|result| {
                if let Some((score, indices)) =
                    self.matcher.fuzzy_indices(&result.display_text, query)
                {
                    let mut scored_result = result.clone();
                    scored_result.score = score;
                    scored_result.match_indices = indices;
                    Some(scored_result)
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (higher is better)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        results
    }
}
