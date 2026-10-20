//! Building and maintaining the browser profile: prefs, the Material 3
//! chrome stylesheets, the extensions folder, and native messaging.

use crate::engine::{self, Engine};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const EXTENSION_ID: &str = "material-3-browser@bhimio1";
pub const UBLOCK_ID: &str = "uBlock0@raymondhill.net";
pub const HOST_NAME: &str = "m3_browser";

const USER_JS: &str = include_str!("../../profile/user.js");
const USER_CHROME: &str = include_str!("../../profile/chrome/userChrome.css");
const USER_CONTENT: &str = include_str!("../../profile/chrome/userContent.css");
const COMPAT: &str = include_str!("../../profile/chrome/compat.css");
const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.otf");
const INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.otf");

/// Where the built extension lives: `extension/` (unpacked) and/or
/// `material-3-browser.xpi` (signed when CI had signing keys).
#[derive(Clone, Debug, Default)]
pub struct Assets {
    pub xpi: Option<PathBuf>,
    pub unpacked: Option<PathBuf>,
}

impl Assets {
    pub fn find() -> Self {
        let mut roots: Vec<PathBuf> = Vec::new();
        if let Some(d) = std::env::var_os("M3_BROWSER_DATA") {
            roots.push(PathBuf::from(d));
        }
        if let Some(exe_dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            roots.push(exe_dir.join("../share/m3-browser"));
            // Running from target/{debug,release} in the repo.
            roots.push(exe_dir.join("../../extension/stage"));
        }
        roots.push(PathBuf::from("/usr/share/m3-browser"));
        roots.push(engine::data_dir());
        Self::find_in(&roots)
    }

    pub fn find_in(roots: &[PathBuf]) -> Self {
        let mut a = Assets::default();
        for root in roots {
            if a.xpi.is_none() {
                let x = root.join("material-3-browser.xpi");
                if x.is_file() {
                    a.xpi = Some(x);
                }
            }
            if a.unpacked.is_none() {
                let d = root.join("extension");
                if d.join("manifest.json").is_file() {
                    a.unpacked = Some(d);
                }
            }
        }
        a
    }
}

/// File names inside a zip archive, from its central directory.
pub fn zip_names(bytes: &[u8]) -> Vec<String> {
    // End of central directory: signature 0x06054b50, at least 22 bytes.
    let Some(eocd) = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| bytes[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
    else {
        return Vec::new();
    };
    let u16_at = |i: usize| bytes.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let u32_at = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let (Some(count), Some(mut pos)) = (u16_at(eocd + 10), u32_at(eocd + 16)) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for _ in 0..count {
        if bytes.get(pos..pos + 4) != Some(&[0x50, 0x4b, 0x01, 0x02]) {
            break;
        }
        let (Some(n), Some(e), Some(c)) = (u16_at(pos + 28), u16_at(pos + 30), u16_at(pos + 32)) else {
            break;
        };
        if let Some(name) = bytes.get(pos + 46..pos + 46 + n) {
            names.push(String::from_utf8_lossy(name).into_owned());
        }
        pos += 46 + n + e + c;
    }
    names
}

/// Mozilla-signed add-ons carry a signature in META-INF.
pub fn is_signed_xpi(path: &Path) -> bool {
    let mut bytes = Vec::new();
    if std::fs::File::open(path).and_then(|mut f| f.read_to_end(&mut bytes)).is_err() {
        return false;
    }
    zip_names(&bytes)
        .iter()
        .any(|n| n == "META-INF/mozilla.rsa" || n == "META-INF/cose.sig")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtensionMode {
    /// Installed permanently from the profile's extensions folder.
    Sideloaded { signed: bool },
    /// Loaded temporarily at each start through the debugging socket.
    DevLoad(PathBuf),
    /// Zen/Firefox release with an unsigned build and dev-load off.
    NeedsSigning,
    /// No built extension was found.
    Missing,
}

/// Decide how the extension gets into the browser.
pub fn extension_mode(engine: &Engine, assets: &Assets, dev_load: bool) -> ExtensionMode {
    let signed = assets.xpi.as_deref().is_some_and(is_signed_xpi);
    if signed {
        return ExtensionMode::Sideloaded { signed: true };
    }
    if !engine.requires_signing && assets.xpi.is_some() {
        return ExtensionMode::Sideloaded { signed: false };
    }
    if dev_load {
        if let Some(src) = assets.unpacked.clone().or_else(|| assets.xpi.clone()) {
            return ExtensionMode::DevLoad(src);
        }
    }
    if assets.xpi.is_none() && assets.unpacked.is_none() {
        ExtensionMode::Missing
    } else {
        ExtensionMode::NeedsSigning
    }
}

/// `user.js`: the shipped template plus prefs that depend on the setup.
pub fn user_js(mode: &ExtensionMode, rdp_socket: Option<&Path>) -> String {
    let mut s = String::from(USER_JS);
    s.push_str("\n// ---- Generated by m3-browser; changes here are overwritten ----\n");
    let unsigned = matches!(mode, ExtensionMode::Sideloaded { signed: false });
    s.push_str(&format!(
        "user_pref(\"xpinstall.signatures.required\", {});\n",
        !unsigned
    ));
    if rdp_socket.is_some() {
        s.push_str("user_pref(\"devtools.debugger.remote-enabled\", true);\n");
        s.push_str("user_pref(\"devtools.chrome.enabled\", true);\n");
        s.push_str("user_pref(\"devtools.debugger.prompt-connection\", false);\n");
    } else {
        s.push_str("user_pref(\"devtools.debugger.remote-enabled\", false);\n");
        s.push_str("user_pref(\"devtools.debugger.prompt-connection\", true);\n");
    }
    s
}

fn write_if_changed(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

/// Create or refresh the profile. Safe to run on every launch.
pub fn prepare(dir: &Path, mode: &ExtensionMode, assets: &Assets, rdp_socket: Option<&Path>) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir.join("chrome/fonts"))?;
    std::fs::create_dir_all(dir.join("extensions"))?;
    write_if_changed(&dir.join("user.js"), user_js(mode, rdp_socket).as_bytes())?;
    write_if_changed(&dir.join("chrome/userChrome.css"), USER_CHROME.as_bytes())?;
    write_if_changed(&dir.join("chrome/userContent.css"), USER_CONTENT.as_bytes())?;
    write_if_changed(&dir.join("chrome/m3-compat.css"), COMPAT.as_bytes())?;
    write_if_changed(&dir.join("chrome/fonts/Inter-Regular.otf"), INTER)?;
    write_if_changed(&dir.join("chrome/fonts/Inter-Medium.otf"), INTER_MEDIUM)?;
    // The user's own additions, imported last by userChrome.css; never overwritten.
    let custom = dir.join("chrome/m3-custom.css");
    if !custom.exists() {
        std::fs::write(&custom, "/* Your own userChrome rules. Material 3 Browser never overwrites this file. */\n")?;
    }

    let target = dir.join("extensions").join(format!("{EXTENSION_ID}.xpi"));
    match (mode, &assets.xpi) {
        (ExtensionMode::Sideloaded { .. }, Some(xpi)) => {
            let bytes = std::fs::read(xpi)?;
            write_if_changed(&target, &bytes)?;
        }
        _ => {
            // A leftover unsigned copy would just show up as "could not be verified".
            if target.exists() && !is_signed_xpi(&target) {
                let _ = std::fs::remove_file(&target);
            }
        }
    }
    Ok(())
}

/// Install the native messaging host manifests and the host script.
pub fn install_native_host(engine: Option<&Engine>, exe: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let data = engine::data_dir();
    std::fs::create_dir_all(&data)?;
    let script = data.join("native-host.sh");
    let quoted = shell_quote(&exe.to_string_lossy());
    write_if_changed(&script, format!("#!/bin/sh\nexec {quoted} --native-host \"$@\"\n").as_bytes())?;
    set_executable(&script)?;

    // Inside the Flatpak sandbox the host has to be started on the host
    // system through flatpak-spawn (needs org.freedesktop.Flatpak access).
    let flatpak_script = if engine.is_some_and(|e| e.flatpak) {
        let var = crate::sys::paths::home().join(".var/app").join(engine::ZEN_FLATPAK_ID).join("data/m3-browser");
        std::fs::create_dir_all(&var)?;
        let s = var.join("native-host.sh");
        write_if_changed(&s, format!("#!/bin/sh\nexec flatpak-spawn --host {quoted} --native-host \"$@\"\n").as_bytes())?;
        set_executable(&s)?;
        Some(s)
    } else {
        None
    };

    let mut written = Vec::new();
    for dir in engine::native_manifest_dirs(engine) {
        let in_sandbox = dir.starts_with(crate::sys::paths::home().join(".var/app"));
        let path = if in_sandbox {
            match &flatpak_script {
                Some(s) => s.clone(),
                None => continue,
            }
        } else {
            script.clone()
        };
        let manifest = serde_json::json!({
            "name": HOST_NAME,
            "description": "Material 3 Browser palette host",
            "path": path,
            "type": "stdio",
            "allowed_extensions": [EXTENSION_ID],
        });
        let file = dir.join(format!("{HOST_NAME}.json"));
        write_if_changed(&file, serde_json::to_string_pretty(&manifest)?.as_bytes())?;
        written.push(file);
    }
    Ok(written)
}

fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

// ------------------------------------------------------------------ import

/// An existing profile we can import bookmarks, history and passwords from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProfile {
    pub browser: &'static str,
    pub name: String,
    pub path: PathBuf,
}

/// Parse a profiles.ini and return profile folders (default first).
pub fn parse_profiles_ini(base: &Path, text: &str) -> Vec<(String, PathBuf, bool)> {
    let mut out = Vec::new();
    let mut name = String::new();
    let mut path = None;
    let mut relative = true;
    let mut default = false;
    let mut install_defaults = Vec::new();
    let mut in_profile = false;
    let flush = |name: &mut String, path: &mut Option<String>, relative: bool, default: bool, out: &mut Vec<(String, PathBuf, bool)>| {
        if let Some(p) = path.take() {
            let full = if relative { base.join(&p) } else { PathBuf::from(&p) };
            out.push((std::mem::take(name), full, default));
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if in_profile {
                flush(&mut name, &mut path, relative, default, &mut out);
            }
            in_profile = line.starts_with("[Profile");
            relative = true;
            default = false;
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match (k.trim(), in_profile) {
            ("Name", true) => name = v.trim().to_string(),
            ("Path", true) => path = Some(v.trim().to_string()),
            ("IsRelative", true) => relative = v.trim() != "0",
            ("Default", true) => default = v.trim() == "1",
            ("Default", false) => install_defaults.push(v.trim().to_string()),
            _ => {}
        }
    }
    if in_profile {
        flush(&mut name, &mut path, relative, default, &mut out);
    }
    // [Install…] Default= wins over the legacy Default=1 flag.
    for (_, p, d) in out.iter_mut() {
        if install_defaults.iter().any(|i| base.join(i) == *p || Path::new(i) == p.as_path()) {
            *d = true;
        }
    }
    out.sort_by_key(|(_, _, d)| !*d);
    out
}

pub fn import_sources() -> Vec<SourceProfile> {
    let home = crate::sys::paths::home();
    let roots: [(&'static str, PathBuf); 5] = [
        ("Zen", home.join(".zen")),
        ("Zen (Flatpak)", home.join(".var/app").join(engine::ZEN_FLATPAK_ID).join(".zen")),
        ("Firefox", home.join(".mozilla/firefox")),
        ("Firefox", crate::sys::paths::config_home().join("mozilla/firefox")),
        ("LibreWolf", home.join(".librewolf")),
    ];
    let mut out = Vec::new();
    for (browser, root) in roots {
        let Ok(text) = std::fs::read_to_string(root.join("profiles.ini")) else { continue };
        for (name, path, _) in parse_profiles_ini(&root, &text) {
            if path.join("places.sqlite").is_file() && !path.ends_with("m3-browser") {
                out.push(SourceProfile { browser, name, path });
            }
        }
    }
    out
}

/// Files copied by an import: bookmarks + history, favicons, saved logins.
pub const IMPORT_FILES: &[&str] = &["places.sqlite", "favicons.sqlite", "logins.json", "key4.db"];

/// Is a Gecko profile in use right now? (`lock` is a symlink to host:+pid.)
pub fn profile_in_use(dir: &Path) -> bool {
    std::fs::symlink_metadata(dir.join("lock")).is_ok()
}

pub fn import(from: &Path, to: &Path) -> Result<Vec<&'static str>, String> {
    if profile_in_use(from) {
        return Err("Close that browser first; its profile is in use.".into());
    }
    if profile_in_use(to) {
        return Err("Close Material 3 Browser first.".into());
    }
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    let mut copied = Vec::new();
    for f in IMPORT_FILES {
        let src = from.join(f);
        if src.is_file() {
            // SQLite write-ahead logs would be stale next to a copied db.
            for suffix in ["-wal", "-shm"] {
                let _ = std::fs::remove_file(to.join(format!("{f}{suffix}")));
            }
            std::fs::copy(&src, to.join(f)).map_err(|e| format!("{f}: {e}"))?;
            copied.push(*f);
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineChoice;

    fn engine(requires_signing: bool) -> Engine {
        Engine { choice: EngineChoice::Zen, label: "Zen", command: vec![], requires_signing, flatpak: false }
    }

    /// A minimal stored zip with the given file names.
    fn make_zip(names: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for n in names {
            let offset = out.len() as u32;
            out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            out.extend_from_slice(&[0; 22]);
            out.extend_from_slice(&(n.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(n.as_bytes());
            central.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            central.extend_from_slice(&[0; 24]);
            central.extend_from_slice(&(n.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&[0; 8]);
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(n.as_bytes());
        }
        let cd_offset = out.len() as u32;
        let cd_len = central.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&cd_len.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn signature_detection() {
        let dir = tempfile::tempdir().unwrap();
        let signed = dir.path().join("s.xpi");
        std::fs::write(&signed, make_zip(&["manifest.json", "META-INF/cose.sig", "META-INF/mozilla.rsa"])).unwrap();
        let plain = dir.path().join("p.xpi");
        std::fs::write(&plain, make_zip(&["manifest.json", "background.js"])).unwrap();
        assert_eq!(zip_names(&std::fs::read(&plain).unwrap()), vec!["manifest.json", "background.js"]);
        assert!(is_signed_xpi(&signed));
        assert!(!is_signed_xpi(&plain));
        assert!(zip_names(b"not a zip").is_empty());
    }

    #[test]
    fn extension_modes() {
        let dir = tempfile::tempdir().unwrap();
        let xpi = dir.path().join("material-3-browser.xpi");
        std::fs::write(&xpi, make_zip(&["manifest.json"])).unwrap();
        let unsigned = Assets { xpi: Some(xpi.clone()), unpacked: None };
        assert_eq!(extension_mode(&engine(true), &unsigned, false), ExtensionMode::NeedsSigning);
        assert_eq!(extension_mode(&engine(true), &unsigned, true), ExtensionMode::DevLoad(xpi.clone()));
        assert_eq!(extension_mode(&engine(false), &unsigned, false), ExtensionMode::Sideloaded { signed: false });
        std::fs::write(&xpi, make_zip(&["manifest.json", "META-INF/cose.sig"])).unwrap();
        assert_eq!(extension_mode(&engine(true), &unsigned, true), ExtensionMode::Sideloaded { signed: true });
        assert_eq!(extension_mode(&engine(true), &Assets::default(), true), ExtensionMode::Missing);
    }

    #[test]
    fn user_js_prefs() {
        let js = user_js(&ExtensionMode::Sideloaded { signed: false }, None);
        assert!(js.contains("toolkit.legacyUserProfileCustomizations.stylesheets\", true"));
        assert!(js.contains("zen.theme.disable-lightweight\", false"));
        assert!(js.contains("xpinstall.signatures.required\", false"));
        assert!(js.contains("devtools.debugger.remote-enabled\", false"));
        let dev = user_js(&ExtensionMode::DevLoad(PathBuf::from("/x")), Some(Path::new("/run/s")));
        assert!(dev.contains("xpinstall.signatures.required\", true"));
        assert!(dev.contains("devtools.debugger.prompt-connection\", false"));
    }

    #[test]
    fn prepare_writes_profile() {
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("profile");
        let xpi = dir.path().join("material-3-browser.xpi");
        std::fs::write(&xpi, make_zip(&["manifest.json"])).unwrap();
        let assets = Assets { xpi: Some(xpi), unpacked: None };
        prepare(&profile, &ExtensionMode::Sideloaded { signed: false }, &assets, None).unwrap();
        assert!(profile.join("chrome/userChrome.css").is_file());
        assert!(profile.join("chrome/fonts/Inter-Regular.otf").is_file());
        assert!(profile.join(format!("extensions/{EXTENSION_ID}.xpi")).is_file());
        std::fs::write(profile.join("chrome/m3-custom.css"), "mine").unwrap();
        // Switching to a mode without sideloading removes the unsigned copy, keeps custom CSS.
        prepare(&profile, &ExtensionMode::NeedsSigning, &assets, None).unwrap();
        assert!(!profile.join(format!("extensions/{EXTENSION_ID}.xpi")).exists());
        assert_eq!(std::fs::read_to_string(profile.join("chrome/m3-custom.css")).unwrap(), "mine");
    }

    #[test]
    fn profiles_ini() {
        let base = Path::new("/home/u/.zen");
        let ini = "[General]\nStartWithLastProfile=1\n\n[Profile1]\nName=work\nIsRelative=1\nPath=abc.work\n\n[Profile0]\nName=Default (release)\nIsRelative=1\nPath=xyz.Default (release)\n\n[Install6ED35B3CA1B5D3E6]\nDefault=xyz.Default (release)\nLocked=1\n";
        let p = parse_profiles_ini(base, ini);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].0, "Default (release)");
        assert!(p[0].2);
        assert_eq!(p[1].1, base.join("abc.work"));
    }

    #[test]
    fn import_copies_known_files() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("from");
        let to = dir.path().join("to");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("places.sqlite"), "p").unwrap();
        std::fs::write(from.join("logins.json"), "l").unwrap();
        std::fs::write(from.join("cookies.sqlite"), "c").unwrap();
        let copied = import(&from, &to).unwrap();
        assert_eq!(copied, vec!["places.sqlite", "logins.json"]);
        assert!(!to.join("cookies.sqlite").exists());
        std::os::unix::fs::symlink("host:+123", from.join("lock")).unwrap();
        assert!(import(&from, &to).is_err());
    }
}
