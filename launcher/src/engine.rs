//! Which Gecko browser runs the profile: Zen (native or Flatpak) first,
//! then LibreWolf and the Firefox channels.

use crate::config::EngineChoice;
use material_3_ui::sys::{cmd, paths};
use std::path::PathBuf;

pub const ZEN_FLATPAK_ID: &str = "app.zen_browser.zen";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Engine {
    pub choice: EngineChoice,
    pub label: &'static str,
    /// Program and leading arguments.
    pub command: Vec<String>,
    /// Release builds only load signed extensions; the others honour
    /// `xpinstall.signatures.required = false`.
    pub requires_signing: bool,
    pub flatpak: bool,
}

struct Candidate {
    choice: EngineChoice,
    label: &'static str,
    programs: &'static [&'static str],
    requires_signing: bool,
}

const NATIVE: &[Candidate] = &[
    Candidate { choice: EngineChoice::Zen, label: "Zen", programs: &["zen-browser", "zen", "zen-bin"], requires_signing: true },
    Candidate { choice: EngineChoice::Librewolf, label: "LibreWolf", programs: &["librewolf"], requires_signing: false },
    Candidate { choice: EngineChoice::FirefoxDeveloper, label: "Firefox Developer Edition", programs: &["firefox-developer-edition"], requires_signing: false },
    Candidate { choice: EngineChoice::FirefoxNightly, label: "Firefox Nightly", programs: &["firefox-nightly"], requires_signing: false },
    Candidate { choice: EngineChoice::FirefoxEsr, label: "Firefox ESR", programs: &["firefox-esr"], requires_signing: false },
    Candidate { choice: EngineChoice::Firefox, label: "Firefox", programs: &["firefox"], requires_signing: true },
];

fn flatpak_installed(id: &str) -> bool {
    cmd::exists("flatpak") && cmd::run("flatpak", &["info", id]).is_ok()
}

fn native(c: &Candidate) -> Option<Engine> {
    let program = c.programs.iter().find(|p| cmd::exists(p))?;
    Some(Engine {
        choice: c.choice,
        label: c.label,
        command: vec![program.to_string()],
        requires_signing: c.requires_signing,
        flatpak: false,
    })
}

fn zen_flatpak() -> Option<Engine> {
    flatpak_installed(ZEN_FLATPAK_ID).then(|| Engine {
        choice: EngineChoice::ZenFlatpak,
        label: "Zen (Flatpak)",
        command: vec!["flatpak".into(), "run".into(), ZEN_FLATPAK_ID.into()],
        requires_signing: true,
        flatpak: true,
    })
}

/// Every engine installed on this machine, in preference order.
pub fn available() -> Vec<Engine> {
    let mut out = Vec::new();
    for c in NATIVE {
        if let Some(e) = native(c) {
            out.push(e);
        }
        if c.choice == EngineChoice::Zen {
            out.extend(zen_flatpak());
        }
    }
    out
}

/// The engine to use for a configured choice.
pub fn resolve(choice: EngineChoice) -> Option<Engine> {
    let all = available();
    match choice {
        EngineChoice::Auto => all.into_iter().next(),
        wanted => all.into_iter().find(|e| e.choice == wanted),
    }
}

/// Default profile folder. The Flatpak can only see its own data folder.
pub fn default_profile_dir(engine: &Engine) -> PathBuf {
    if engine.flatpak {
        paths::home().join(".var/app").join(ZEN_FLATPAK_ID).join(".zen/m3-browser")
    } else {
        data_dir().join("profile")
    }
}

/// `~/.local/share/m3-browser`.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| paths::home().join(".local/share"))
        .join("m3-browser")
}

/// Folders Gecko browsers read native messaging manifests from. Firefox
/// uses `~/.mozilla`; Zen and LibreWolf builds use their own dot folders,
/// and newer Firefox also honours XDG config.
pub fn native_manifest_dirs(engine: Option<&Engine>) -> Vec<PathBuf> {
    let home = paths::home();
    let mut dirs = vec![
        home.join(".mozilla/native-messaging-hosts"),
        home.join(".zen/native-messaging-hosts"),
        home.join(".librewolf/native-messaging-hosts"),
        paths::config_home().join("mozilla/native-messaging-hosts"),
    ];
    if engine.is_some_and(|e| e.flatpak) {
        let var = home.join(".var/app").join(ZEN_FLATPAK_ID);
        dirs.push(var.join(".mozilla/native-messaging-hosts"));
        dirs.push(var.join(".zen/native-messaging-hosts"));
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_requirements() {
        let zen = NATIVE.iter().find(|c| c.choice == EngineChoice::Zen).unwrap();
        assert!(zen.requires_signing);
        let lw = NATIVE.iter().find(|c| c.choice == EngineChoice::Librewolf).unwrap();
        assert!(!lw.requires_signing);
    }

    #[test]
    fn flatpak_profile_is_inside_its_data_dir() {
        let e = Engine {
            choice: EngineChoice::ZenFlatpak,
            label: "Zen (Flatpak)",
            command: vec![],
            requires_signing: true,
            flatpak: true,
        };
        let dir = default_profile_dir(&e);
        assert!(dir.to_string_lossy().contains(".var/app/app.zen_browser.zen/"));
        assert!(native_manifest_dirs(Some(&e)).iter().any(|d| d.starts_with(paths::home().join(".var/app"))));
    }
}
