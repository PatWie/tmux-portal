use anyhow::Result;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub colors: ColorConfig,
    pub session_order: Option<Vec<String>>,
    #[serde(default)]
    pub line_numbers: LineNumberConfig,
    #[serde(default)]
    pub search_paths: Vec<String>, // Legacy support
    #[serde(default)]
    pub search_patterns: Vec<SearchPatternConfig>,
    #[serde(default)]
    pub show_window_ids: bool, // Show window IDs when names are ambiguous
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPatternConfig {
    pub name: String,
    pub paths: Vec<String>,
    pub pattern: String,
}

impl Default for SearchPatternConfig {
    fn default() -> Self {
        Self {
            name: "git-style".to_string(),
            paths: Vec::new(),
            pattern: "{session}/{window}".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LineNumberConfig {
    pub padding: usize,
    pub current_line_color: StyleConfig,
    pub other_lines_color: StyleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub session: StyleConfig,
    pub window_active: StyleConfig,
    pub window_inactive: StyleConfig,
    pub window_selected: StyleConfig,
    pub session_selected: StyleConfig,
    pub border_list: StyleConfig,
    pub border_prompt: StyleConfig,
    pub border_search: StyleConfig,
    pub help_key: StyleConfig,
    pub help_text: StyleConfig,
    pub status_text: StyleConfig,
    pub error_text: StyleConfig,
    pub popup_border: StyleConfig,
    pub popup_input: StyleConfig,
    #[serde(default = "default_popup_text")]
    pub popup_text: StyleConfig,
    // New configurable colors with default fallback
    #[serde(default = "default_numeric_buffer")]
    pub numeric_buffer: StyleConfig,
    #[serde(default = "default_search_highlight")]
    pub search_highlight: StyleConfig,
    #[serde(default = "default_quick_search_match")]
    pub quick_search_match: StyleConfig,
    #[serde(default = "default_quick_search_selected")]
    pub quick_search_selected: StyleConfig,
    #[serde(default = "default_list_highlight")]
    pub list_highlight: StyleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct StyleConfig {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub dim: Option<bool>,
    pub reversed: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            colors: ColorConfig::default(),
            session_order: None,
            line_numbers: LineNumberConfig::default(),
            search_paths: Vec::new(),
            search_patterns: Vec::new(),
            show_window_ids: true, // Default to showing IDs for disambiguation
        }
    }
}

impl Default for LineNumberConfig {
    fn default() -> Self {
        Self {
            padding: 5,
            current_line_color: StyleConfig {
                fg: Some("dark_gray".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            other_lines_color: StyleConfig {
                fg: Some("dark_gray".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
        }
    }
}

// Default functions for new color configurations
fn default_numeric_buffer() -> StyleConfig {
    StyleConfig {
        fg: Some("cyan".to_string()),
        bg: None,
        bold: Some(true),
        italic: None,
        underline: None,
        dim: None,
        reversed: None,
    }
}

fn default_popup_text() -> StyleConfig {
    StyleConfig {
        fg: Some("white".to_string()),
        bg: None,
        bold: None,
        italic: None,
        underline: None,
        dim: None,
        reversed: None,
    }
}

fn default_search_highlight() -> StyleConfig {
    StyleConfig {
        fg: Some("cyan".to_string()),
        bg: None,
        bold: Some(true),
        italic: None,
        underline: None,
        dim: None,
        reversed: None,
    }
}

fn default_quick_search_match() -> StyleConfig {
    StyleConfig {
        fg: None,
        bg: None,
        bold: Some(true),
        italic: None,
        underline: None,
        dim: None,
        reversed: None,
    }
}

fn default_quick_search_selected() -> StyleConfig {
    StyleConfig {
        fg: Some("black".to_string()),
        bg: Some("cyan".to_string()),
        bold: Some(true),
        italic: None,
        underline: None,
        dim: None,
        reversed: None,
    }
}

fn default_list_highlight() -> StyleConfig {
    StyleConfig {
        fg: None,
        bg: None,
        bold: None,
        italic: None,
        underline: None,
        dim: None,
        reversed: Some(true),
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            session: StyleConfig {
                fg: None,
                bg: None,
                bold: Some(true),
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            window_active: StyleConfig {
                fg: None,
                bg: None,
                bold: Some(false),
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            window_inactive: StyleConfig {
                fg: None,
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            window_selected: StyleConfig {
                fg: Some("yellow".to_string()),
                bg: Some("#1b2433".to_string()),
                bold: Some(true),
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            session_selected: StyleConfig {
                fg: None,
                bg: Some("light_blue".to_string()),
                bold: Some(true),
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            border_list: StyleConfig {
                fg: Some("white".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            border_prompt: StyleConfig {
                fg: Some("yellow".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            border_search: StyleConfig {
                fg: Some("cyan".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            help_key: StyleConfig {
                fg: Some("yellow".to_string()),
                bg: None,
                bold: Some(true),
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            help_text: StyleConfig {
                fg: Some("white".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            status_text: StyleConfig {
                fg: Some("green".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            error_text: StyleConfig {
                fg: Some("red".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            popup_border: StyleConfig {
                fg: Some("yellow".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            popup_input: StyleConfig {
                fg: Some("white".to_string()),
                bg: None,
                bold: None,
                italic: None,
                underline: None,
                dim: None,
                reversed: None,
            },
            popup_text: default_popup_text(),
            // New configurable colors using default functions
            numeric_buffer: default_numeric_buffer(),
            search_highlight: default_search_highlight(),
            quick_search_match: default_quick_search_match(),
            quick_search_selected: default_quick_search_selected(),
            list_highlight: default_list_highlight(),
        }
    }
}

impl StyleConfig {
    pub fn to_ratatui_style(&self) -> Style {
        let mut style = Style::default();

        if let Some(fg_str) = &self.fg {
            style = style.fg(parse_color(fg_str));
        }

        if let Some(bg_str) = &self.bg {
            style = style.bg(parse_color(bg_str));
        }

        let mut modifiers = Modifier::empty();

        if self.bold.unwrap_or(false) {
            modifiers |= Modifier::BOLD;
        }
        if self.italic.unwrap_or(false) {
            modifiers |= Modifier::ITALIC;
        }
        if self.underline.unwrap_or(false) {
            modifiers |= Modifier::UNDERLINED;
        }
        if self.dim.unwrap_or(false) {
            modifiers |= Modifier::DIM;
        }
        if self.reversed.unwrap_or(false) {
            modifiers |= Modifier::REVERSED;
        }

        if !modifiers.is_empty() {
            style = style.add_modifier(modifiers);
        }

        style
    }
}

fn parse_color(color_str: &str) -> Color {
    match color_str.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        // Try to parse as RGB hex (e.g., "#FF0000", "FF0000", or "0xFF0000")
        hex if hex.starts_with('#') && hex.len() == 7 => {
            if let Ok(rgb) = u32::from_str_radix(&hex[1..], 16) {
                Color::Rgb(
                    ((rgb >> 16) & 0xFF) as u8,
                    ((rgb >> 8) & 0xFF) as u8,
                    (rgb & 0xFF) as u8,
                )
            } else {
                Color::White
            }
        }
        hex if hex.starts_with("0x") && hex.len() == 8 => {
            if let Ok(rgb) = u32::from_str_radix(&hex[2..], 16) {
                Color::Rgb(
                    ((rgb >> 16) & 0xFF) as u8,
                    ((rgb >> 8) & 0xFF) as u8,
                    (rgb & 0xFF) as u8,
                )
            } else {
                Color::White
            }
        }
        hex if hex.len() == 6 => {
            if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                Color::Rgb(
                    ((rgb >> 16) & 0xFF) as u8,
                    ((rgb >> 8) & 0xFF) as u8,
                    (rgb & 0xFF) as u8,
                )
            } else {
                Color::White
            }
        }
        // Try to parse as 256-color index
        num_str => {
            if let Ok(index) = num_str.parse::<u8>() {
                Color::Indexed(index)
            } else {
                Color::White
            }
        }
    }
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    let config = if config_path.exists() {
        // Load existing config
        let config_str = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_str)?;

        // Check if we need to update the config with new defaults
        let default_config = Config::default();
        let serialized_config = toml::to_string(&config)?;
        let serialized_default = toml::to_string(&default_config)?;

        // Only write back if the config would be different (missing fields that need defaults)
        if serialized_config != serialized_default && !has_all_fields(&config, &default_config) {
            write_config(&config_path, &config)?;
        }

        config
    } else {
        // Create default config for first time use
        let default_config = Config::default();
        write_config(&config_path, &default_config)?;
        default_config
    };

    Ok(config)
}

fn write_config(path: &PathBuf, config: &Config) -> Result<()> {
    let config_str = toml::to_string(config)?;
    fs::write(path, config_str)?;
    Ok(())
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;

    let tmux_portal_config_dir = config_dir.join("tmux_portal");
    if !tmux_portal_config_dir.exists() {
        fs::create_dir_all(&tmux_portal_config_dir)?;
    }

    Ok(tmux_portal_config_dir.join("config.toml"))
}

// Check if the loaded config has all fields from the default config
// This is a simple check to determine if we need to write back the config
fn has_all_fields(config: &Config, default_config: &Config) -> bool {
    // Convert both configs to toml::Value for easier comparison
    let config_value = match toml::to_string(config) {
        Ok(s) => match toml::from_str::<toml::Value>(&s) {
            Ok(v) => v,
            Err(_) => return false,
        },
        Err(_) => return false,
    };

    let default_value = match toml::to_string(default_config) {
        Ok(s) => match toml::from_str::<toml::Value>(&s) {
            Ok(v) => v,
            Err(_) => return false,
        },
        Err(_) => return false,
    };

    // Check if all fields in default are present in config
    has_all_fields_recursive(&config_value, &default_value)
}

fn has_all_fields_recursive(config: &toml::Value, default: &toml::Value) -> bool {
    match (config, default) {
        (toml::Value::Table(config_map), toml::Value::Table(default_map)) => {
            // Check if all keys in default_map exist in config_map
            for (key, default_value) in default_map {
                match config_map.get(key) {
                    Some(config_value) => {
                        // Recursively check nested structures
                        if !has_all_fields_recursive(config_value, default_value) {
                            return false;
                        }
                    }
                    None => return false, // Missing key
                }
            }
            true
        }
        // For non-table types, just check if they're the same type
        _ => std::mem::discriminant(config) == std::mem::discriminant(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("RED"), Color::Red);
        assert_eq!(parse_color("#FF0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("FF0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("0xFF0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("0x11161f"), Color::Rgb(17, 22, 31));
        assert_eq!(parse_color("42"), Color::Indexed(42));
        assert_eq!(parse_color("invalid"), Color::White);
    }
}
