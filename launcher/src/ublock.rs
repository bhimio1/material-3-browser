//! uBlock Origin, pre-installed. The signed xpi comes straight from
//! addons.mozilla.org into the profile's extensions folder; once there,
//! Firefox/Zen keep it updated through AMO like any other add-on.

use crate::profile::{is_signed_xpi, zip_names, UBLOCK_ID};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const URL: &str = "https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/latest.xpi";

pub fn target(profile: &Path) -> PathBuf {
    profile.join("extensions").join(format!("{UBLOCK_ID}.xpi"))
}

pub fn installed(profile: &Path) -> bool {
    target(profile).is_file()
}

/// Fetch a URL. `curl` (always present on Arch) honours proxies and the
/// system certificate store; ureq with the proxy from the environment is the
/// fallback.
fn download(url: &str) -> Result<Vec<u8>, String> {
    if crate::sys::cmd::exists("curl") {
        let out = std::process::Command::new("curl")
            .args(["-fsSL", "--max-time", "120", "--max-filesize", "67108864", url])
            .output()
            .map_err(|e| format!("curl: {e}"))?;
        if out.status.success() {
            return Ok(out.stdout);
        }
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        log::warn!("curl failed ({err}), trying the built-in client");
    }
    let agent = ureq::AgentBuilder::new()
        .try_proxy_from_env(true)
        .timeout(std::time::Duration::from_secs(60))
        .build();
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// Download and install. Refuses anything that isn't a signed xpi.
pub fn install(profile: &Path) -> Result<(), String> {
    let bytes = download(URL).map_err(|e| format!("Couldn't download uBlock Origin: {e}"))?;
    if !zip_names(&bytes).iter().any(|n| n == "manifest.json") {
        return Err("The uBlock Origin download wasn't an add-on package.".into());
    }
    let dest = target(profile);
    std::fs::create_dir_all(dest.parent().unwrap_or(profile)).map_err(|e| e.to_string())?;
    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    if !is_signed_xpi(&tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err("The uBlock Origin download isn't signed by Mozilla; not installing it.".into());
    }
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())
}

pub fn remove(profile: &Path) {
    let _ = std::fs::remove_file(target(profile));
}
