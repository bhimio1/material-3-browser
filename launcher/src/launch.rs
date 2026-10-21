//! Putting it together: pick the engine, refresh the profile, install the
//! native host, and start the browser.

use crate::config::Config;
use crate::engine::{self, Engine};
use crate::profile::{self, Assets, ExtensionMode};
use crate::{devload, ublock};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

pub const APP_ID: &str = "material_3_browser";

#[derive(Clone, Debug)]
pub struct Plan {
    pub engine: Engine,
    pub profile_dir: PathBuf,
    pub assets: Assets,
    pub mode: ExtensionMode,
    pub rdp_socket: Option<PathBuf>,
}

pub fn plan(config: &Config) -> Result<Plan, String> {
    let engine = engine::resolve(config.engine).ok_or_else(|| {
        "No supported browser found. Install Zen (zen-browser-bin from the AUR, or the app.zen_browser.zen Flatpak), LibreWolf or Firefox.".to_string()
    })?;
    let profile_dir = if config.profile_dir.trim().is_empty() {
        engine::default_profile_dir(&engine)
    } else {
        material_3_ui::sys::paths::expand(config.profile_dir.trim())
    };
    let assets = Assets::find();
    // Dev-load needs a socket the browser can create; not inside the Flatpak.
    let dev_load = config.dev_load && !engine.flatpak;
    let mode = profile::extension_mode(&engine, &assets, dev_load);
    let rdp_socket = matches!(mode, ExtensionMode::DevLoad(_)).then(devload::socket_path);
    Ok(Plan { engine, profile_dir, assets, mode, rdp_socket })
}

/// Refresh the profile and native host. Returns non-fatal warnings.
pub fn prepare(plan: &Plan, config: &Config) -> Result<Vec<String>, String> {
    let mut warnings = Vec::new();
    if plan.rdp_socket.is_some() {
        devload::prepare_socket_dir().map_err(|e| e.to_string())?;
    }
    profile::prepare(&plan.profile_dir, &plan.mode, &plan.assets, plan.rdp_socket.as_deref())
        .map_err(|e| format!("Couldn't prepare the profile: {e}"))?;
    match std::env::current_exe() {
        Ok(exe) => {
            if let Err(e) = profile::install_native_host(Some(&plan.engine), &exe) {
                warnings.push(format!("Palette host not installed: {e}"));
            }
        }
        Err(e) => warnings.push(format!("Palette host not installed: {e}")),
    }
    if config.ublock && !ublock::installed(&plan.profile_dir) {
        if let Err(e) = ublock::install(&plan.profile_dir) {
            warnings.push(e);
        }
    } else if !config.ublock {
        ublock::remove(&plan.profile_dir);
    }
    match &plan.mode {
        ExtensionMode::NeedsSigning => warnings.push(format!(
            "{} only runs signed add-ons and this build of the extension isn't signed yet. Turn on development loading in m3-browser --settings, or use LibreWolf.",
            plan.engine.label
        )),
        ExtensionMode::Missing => warnings.push("The Material 3 Browser extension wasn't found next to m3-browser.".into()),
        _ => {}
    }
    Ok(warnings)
}

pub fn command(plan: &Plan, urls: &[String]) -> Command {
    let mut cmd = Command::new(&plan.engine.command[0]);
    cmd.args(&plan.engine.command[1..]);
    cmd.arg("--profile").arg(&plan.profile_dir);
    cmd.args(["--name", APP_ID, "--class", APP_ID]);
    if let Some(sock) = &plan.rdp_socket {
        cmd.arg("--start-debugger-server").arg(sock);
    }
    // Extra engine arguments for debugging, e.g. "--marionette".
    if let Ok(extra) = std::env::var("M3_BROWSER_ENGINE_ARGS") {
        cmd.args(extra.split_whitespace());
    }
    cmd.args(urls);
    cmd.env("MOZ_ENABLE_WAYLAND", "1");
    cmd
}

/// Start the browser. With dev-load on, waits for it and loads the extension.
pub fn launch(plan: &Plan, urls: &[String]) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let mut cmd = command(plan, urls);
    cmd.process_group(0);
    cmd.spawn().map_err(|e| format!("Couldn't start {}: {e}", plan.engine.label))?;
    if let (ExtensionMode::DevLoad(src), Some(sock)) = (&plan.mode, &plan.rdp_socket) {
        devload::load_when_ready(sock, src, Duration::from_secs(45))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineChoice;

    #[test]
    fn command_line() {
        let plan = Plan {
            engine: Engine {
                choice: EngineChoice::ZenFlatpak,
                label: "Zen (Flatpak)",
                command: vec!["flatpak".into(), "run".into(), "app.zen_browser.zen".into()],
                requires_signing: true,
                flatpak: true,
            },
            profile_dir: PathBuf::from("/p"),
            assets: Assets::default(),
            mode: ExtensionMode::NeedsSigning,
            rdp_socket: None,
        };
        let cmd = command(&plan, &["https://example.org".into()]);
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(cmd.get_program(), "flatpak");
        assert_eq!(
            args,
            vec!["run", "app.zen_browser.zen", "--profile", "/p", "--name", APP_ID, "--class", APP_ID, "https://example.org"]
        );
    }
}
