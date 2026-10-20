//! The launcher's own settings (`~/.config/m3-browser/config.toml`). The
//! extension keeps its settings inside the browser profile.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EngineChoice {
    /// Zen, then LibreWolf, then Firefox.
    #[default]
    Auto,
    Zen,
    ZenFlatpak,
    Librewolf,
    Firefox,
    FirefoxDeveloper,
    FirefoxNightly,
    FirefoxEsr,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSource {
    /// end-4's colors.json when present, else the seed color.
    #[default]
    Auto,
    Seed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub engine: EngineChoice,
    /// Profile folder override. Empty = the default for the engine.
    pub profile_dir: String,
    /// Install the extension temporarily on every start through the remote
    /// debugging protocol. Needed only for unsigned builds on Zen/Firefox
    /// release; opens a local debugging socket while the browser runs.
    pub dev_load: bool,
    /// Keep uBlock Origin installed in the profile.
    pub ublock: bool,
    /// Unix time of the last uBlock Origin download.
    pub ublock_checked: i64,
    pub theme_source: ThemeSource,
    pub color_scheme: ColorScheme,
    pub seed_color: String,
    pub ui_scale: f32,
    /// The setup window was shown once.
    pub setup_done: bool,
    /// Grant the Zen Flatpak access to flatpak-spawn so the palette host
    /// can run outside the sandbox.
    pub flatpak_host_access: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            engine: EngineChoice::Auto,
            profile_dir: String::new(),
            dev_load: false,
            ublock: true,
            ublock_checked: 0,
            theme_source: ThemeSource::Auto,
            color_scheme: ColorScheme::System,
            seed_color: crate::theme::palette::DEFAULT_SEED.into(),
            ui_scale: 1.0,
            setup_done: false,
            flatpak_host_access: false,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("m3-browser/config.toml"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path().ok_or_else(|| anyhow::anyhow!("no config directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_configs_load() {
        let c: Config = toml::from_str("engine = \"zen-flatpak\"\ndev_load = true\n").unwrap();
        assert_eq!(c.engine, EngineChoice::ZenFlatpak);
        assert!(c.dev_load && c.ublock);
        let back: Config = toml::from_str(&toml::to_string_pretty(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }
}
