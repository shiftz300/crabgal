// Game configuration, loaded from config.yaml.
// Inspired by Raven's MainConfig pattern.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{collections::HashMap, fs};

/// Top-level game configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    /// Game title displayed in window title bar.
    #[serde(default = "default_title")]
    pub title: String,

    /// Background asset alias or path used by the dedicated title screen.
    #[serde(default = "default_title_background")]
    pub title_background: String,

    /// Asset path mappings (key → relative path under assets/).
    #[serde(default)]
    pub assets: AssetMap,

    /// Font configuration.
    #[serde(default)]
    pub fonts: FontConfig,

    /// UI style overrides.
    #[serde(default)]
    pub styles: StyleConfig,

    /// Layout settings (anchor offsets, dodge, etc).
    #[serde(default)]
    pub layout: LayoutConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Default offset from left/right screen edge for sprites (design px).
    #[serde(default = "default_anchor_offset")]
    pub anchor_offset: f32,
    /// Height of standing sprites in design pixels.
    #[serde(default = "default_sprite_height")]
    pub sprite_height: f32,

    // ── Textbox positioning (percent of 2560x1440 design area) ──
    /// Textbox left edge when no dodge (%).
    #[serde(default = "default_textbox_left")]
    pub textbox_left: f32,
    /// Textbox left edge when dodged (%).
    #[serde(default = "default_textbox_dodge_left")]
    pub textbox_dodge_left: f32,
    /// Textbox distance from bottom (%).
    #[serde(default = "default_textbox_bottom")]
    pub textbox_bottom: f32,
    /// Textbox height (%).
    #[serde(default = "default_textbox_height")]
    pub textbox_height: f32,
    /// Name bar distance from bottom (%).
    #[serde(default = "default_namebar_bottom")]
    pub namebar_bottom: f32,
}

fn default_anchor_offset() -> f32 {
    40.0
}
fn default_sprite_height() -> f32 {
    1100.0
}
fn default_textbox_left() -> f32 {
    7.0
}
fn default_textbox_dodge_left() -> f32 {
    10.0
}
fn default_textbox_bottom() -> f32 {
    1.0
}
fn default_textbox_height() -> f32 {
    22.0
}
fn default_namebar_bottom() -> f32 {
    24.0
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            anchor_offset: default_anchor_offset(),
            sprite_height: default_sprite_height(),
            textbox_left: default_textbox_left(),
            textbox_dodge_left: default_textbox_dodge_left(),
            textbox_bottom: default_textbox_bottom(),
            textbox_height: default_textbox_height(),
            namebar_bottom: default_namebar_bottom(),
        }
    }
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
    /// Speaker name size (design px).
    #[serde(default = "default_speaker_size")]
    pub speaker_size: f32,
    /// Dialogue text size (design px).
    #[serde(default = "default_dialogue_size")]
    pub dialogue_size: f32,
    /// Control bar icon size (design px).
    #[serde(default = "default_icon_size")]
    pub icon_size: f32,
    /// Control bar label size (design px).
    #[serde(default = "default_label_size")]
    pub label_size: f32,
}

fn default_speaker_size() -> f32 {
    52.0
}
fn default_dialogue_size() -> f32 {
    60.0
}
fn default_icon_size() -> f32 {
    26.0
}
fn default_label_size() -> f32 {
    24.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    /// Textbox background opacity (0.0–1.0).
    #[serde(default = "default_textbox_alpha")]
    pub textbox_alpha: f32,
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
fn default_title_background() -> String {
    "bg".into()
}
fn default_textbox_alpha() -> f32 {
    0.72
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
            title_background: default_title_background(),
            assets: AssetMap::default(),
            fonts: FontConfig::default(),
            styles: StyleConfig::default(),
            layout: LayoutConfig::default(),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            speaker_size: default_speaker_size(),
            dialogue_size: default_dialogue_size(),
            icon_size: default_icon_size(),
            label_size: default_label_size(),
        }
    }
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            textbox_alpha: default_textbox_alpha(),
            typewriter_speed: default_typewriter_speed(),
            auto_delay: default_auto_delay(),
        }
    }
}

impl GameConfig {
    /// Load from a YAML file, falling back to defaults.
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(yaml) => serde_yaml::from_str(&yaml).unwrap_or_else(|error| {
                log::error!("invalid config {}: {error}; using defaults", path.display());
                Self::default()
            }),
            Err(error) => {
                log::warn!(
                    "failed to read config {}: {error}; using defaults",
                    path.display()
                );
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

    /// Resolve a voice asset name to its path below the local asset root.
    pub fn voice_path(&self, name: &str) -> String {
        self.assets
            .voices
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("vocal/{name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = GameConfig::default();
        assert_eq!(cfg.title, "crabgal");
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
    }
}
