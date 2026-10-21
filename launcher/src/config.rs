//! The launcher's own settings (`~/.config/m3-browser/config.toml`). The
//! extension keeps its settings inside the browser profile.

use material_3_ui::theme::{ColorScheme, ThemeSettings, ThemeSource};
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
    /// The setup window was shown once.
    pub setup_done: bool,
    /// Grant the Zen Flatpak access to flatpak-spawn so the palette host
    /// can run outside the sandbox.
    pub flatpak_host_access: bool,
    /// Color source, light/dark, seed color, icons and scale, shared with
    /// the other Material 3 apps. Websites and the browser use this
    /// palette too. Last, as a TOML table.
    pub appearance: ThemeSettings,
}

/// The appearance fields as 0.1 stored them, at the top level.
#[derive(Deserialize)]
struct Legacy {
    theme_source: Option<ThemeSource>,
    color_scheme: Option<ColorScheme>,
    seed_color: Option<String>,
    ui_scale: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            engine: EngineChoice::Auto,
            profile_dir: String::new(),
            dev_load: false,
            ublock: true,
            ublock_checked: 0,
            setup_done: false,
            flatpak_host_access: false,
            appearance: ThemeSettings::default(),
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
            .and_then(|s| Self::parse(&s))
            .unwrap_or_default()
    }

    /// Reads the file, moving 0.1's top-level appearance fields into
    /// `appearance`.
    fn parse(text: &str) -> Option<Self> {
        let mut config: Config = toml::from_str(text).ok()?;
        let table: toml::Table = toml::from_str(text).ok()?;
        if !table.contains_key("appearance") {
            if let Ok(old) = toml::from_str::<Legacy>(text) {
                let a = &mut config.appearance;
                a.source = old.theme_source.unwrap_or(a.source);
                a.scheme = old.color_scheme.unwrap_or(a.scheme);
                a.seed_color = old.seed_color.unwrap_or(std::mem::take(&mut a.seed_color));
                a.ui_scale = old.ui_scale.unwrap_or(a.ui_scale);
            }
        }
        Some(config)
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
        let c = Config::parse("engine = \"zen-flatpak\"\ndev_load = true\n").unwrap();
        assert_eq!(c.engine, EngineChoice::ZenFlatpak);
        assert!(c.dev_load && c.ublock);
        let back = Config::parse(&toml::to_string_pretty(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn old_appearance_fields_carry_over() {
        let old = "engine = \"zen\"\ntheme_source = \"seed\"\ncolor_scheme = \"light\"\nseed_color = \"#C2185B\"\nui_scale = 1.25\nsetup_done = true\n";
        let c = Config::parse(old).unwrap();
        assert_eq!(c.appearance.source, ThemeSource::Seed);
        assert_eq!(c.appearance.scheme, ColorScheme::Light);
        assert_eq!(c.appearance.seed_color, "#C2185B");
        assert_eq!(c.appearance.ui_scale, 1.25);
        assert!(c.appearance.filled_icons);
        assert!(c.setup_done);
        let text = toml::to_string_pretty(&c).unwrap();
        assert!(text.contains("[appearance]"));
        assert_eq!(Config::parse(&text).unwrap(), c);
        let c = Config::parse("color_scheme = \"dark\"\n[appearance]\nscheme = \"light\"\n").unwrap();
        assert_eq!(c.appearance.scheme, ColorScheme::Light);
    }
}
