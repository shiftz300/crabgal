// Game configuration, loaded from config.yaml.
// Inspired by Raven's MainConfig pattern.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level game configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    /// Game title displayed in window title bar.
    #[serde(default = "default_title")]
    pub title: String,

    /// Design resolution (default 2560x1440).
    #[serde(default = "default_resolution")]
    pub resolution: Resolution,

    /// Asset path mappings (key → relative path under assets/).
    #[serde(default)]
    pub assets: AssetMap,

    /// Font configuration.
    #[serde(default)]
    pub fonts: FontConfig,

    /// UI style overrides.
    #[serde(default)]
    pub styles: StyleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    #[serde(default = "default_w")]
    pub width: f32,
    #[serde(default = "default_h")]
    pub height: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetMap {
    /// Background name → file path.
    #[serde(default)]
    pub backgrounds: HashMap<String, String>,
    /// Figure/character name → file path.
    #[serde(default)]
    pub figures: HashMap<String, String>,
    /// BGM name → file path.
    #[serde(default)]
    pub bgm: HashMap<String, String>,
    /// Voice name → file path.
    #[serde(default)]
    pub voices: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Default text size in design pixels.
    #[serde(default = "default_font_size")]
    pub size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    /// Textbox background opacity (0.0–1.0).
    #[serde(default = "default_textbox_alpha")]
    pub textbox_alpha: f32,
    /// Text color as CSS string (e.g. "#ffffff").
    #[serde(default = "default_text_color")]
    pub text_color: String,
    /// Name color.
    #[serde(default = "default_name_color")]
    pub name_color: String,
    /// Typewriter speed in chars per second.
    #[serde(default = "default_typewriter_speed")]
    pub typewriter_speed: f64,
    /// Auto-play delay in seconds.
    #[serde(default = "default_auto_delay")]
    pub auto_delay: f64,
}

// ── Defaults ──

fn default_title() -> String {
    "crabgal".into()
}
fn default_resolution() -> Resolution {
    Resolution {
        width: 2560.0,
        height: 1440.0,
    }
}
fn default_w() -> f32 {
    2560.0
}
fn default_h() -> f32 {
    1440.0
}
fn default_font_size() -> f32 {
    44.0
}
fn default_textbox_alpha() -> f32 {
    0.72
}
fn default_text_color() -> String {
    "#ffffff".into()
}
fn default_name_color() -> String {
    "#ffffff".into()
}
fn default_typewriter_speed() -> f64 {
    45.0
}
fn default_auto_delay() -> f64 {
    2.0
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            resolution: default_resolution(),
            assets: AssetMap::default(),
            fonts: FontConfig::default(),
            styles: StyleConfig::default(),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            size: default_font_size(),
        }
    }
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            textbox_alpha: default_textbox_alpha(),
            text_color: default_text_color(),
            name_color: default_name_color(),
            typewriter_speed: default_typewriter_speed(),
            auto_delay: default_auto_delay(),
        }
    }
}

impl GameConfig {
    /// Load from a YAML file, falling back to defaults.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(yaml) => serde_yaml::from_str(&yaml).unwrap_or_default(),
            Err(_) => {
                log::warn!("No config.yaml found at {:?}, using defaults", path);
                Self::default()
            }
        }
    }

    /// Resolve a background asset name to its file path.
    pub fn bg_path(&self, name: &str) -> String {
        self.assets
            .backgrounds
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("background/{}", name))
    }

    /// Resolve a figure asset name to its file path.
    pub fn figure_path(&self, name: &str) -> String {
        self.assets
            .figures
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("figure/{}", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = GameConfig::default();
        assert_eq!(cfg.title, "crabgal");
        assert_eq!(cfg.resolution.width, 2560.0);
        assert_eq!(cfg.styles.typewriter_speed, 45.0);
    }

    #[test]
    fn test_parse_minimal() {
        let yaml = r#"
title: "Test Game"
styles:
  typewriter_speed: 60.0
"#;
        let cfg: GameConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.title, "Test Game");
        assert_eq!(cfg.styles.typewriter_speed, 60.0);
        assert_eq!(cfg.resolution.width, 2560.0); // default
    }
}
