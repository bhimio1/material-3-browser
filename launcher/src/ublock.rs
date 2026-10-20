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

/// Download and install. Refuses anything that isn't a signed xpi.
pub fn install(profile: &Path) -> Result<(), String> {
    let resp = ureq::get(URL)
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| format!("Couldn't download uBlock Origin: {e}"))?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
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
