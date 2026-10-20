//! Native messaging host (`m3-browser --native-host`): sends the Material 3
//! palette to the extension and pushes updates when end-4 regenerates its
//! colors. Messages are JSON with a 4-byte little-endian length prefix.
//!
//! Extension → host: `{"type":"hello"}`, `{"type":"wallpaper","maxWidth":1920}`
//! Host → extension: `{"type":"palette",…}` (see [`palette_message`]),
//! `{"type":"wallpaper","dataUrl":…}`.

use crate::config::{ColorScheme, Config, ThemeSource};
use crate::theme::{sources, M3Palette};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

/// Firefox caps host → extension messages at 1 MB.
pub const MAX_OUTGOING: usize = 1024 * 1024;

pub fn read_message(r: &mut impl Read) -> std::io::Result<Option<Value>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes(len) as usize;
    if len > 64 * 1024 * 1024 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "message too large"));
    }
    let mut buf = vec![0; len];
    r.read_exact(&mut buf)?;
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn write_message(w: &mut impl Write, msg: &Value) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(msg)?;
    if bytes.len() > MAX_OUTGOING {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "message over 1 MB"));
    }
    w.write_all(&(bytes.len() as u32).to_le_bytes())?;
    w.write_all(&bytes)?;
    w.flush()
}

fn roles(p: &M3Palette) -> Value {
    let map: serde_json::Map<String, Value> = p
        .to_hex_map()
        .into_iter()
        .map(|(k, v)| (k.to_string(), Value::String(v)))
        .collect();
    Value::Object(map)
}

/// The palette in both schemes plus which one the desktop uses.
pub fn palette_message(config: &Config, colors_file: Option<PathBuf>, system_dark: Option<bool>) -> Value {
    let file = match config.theme_source {
        ThemeSource::Auto => colors_file.filter(|p| p.is_file()),
        ThemeSource::Seed => None,
    };
    if let Some(path) = &file {
        if let Ok(current) = M3Palette::from_theme_json(path) {
            let dark = current.is_dark();
            let seed = crate::theme::palette::to_hex(current.primary);
            let other = M3Palette::from_seed(&seed, !dark);
            let (d, l) = if dark { (current, other) } else { (other, current) };
            return json!({
                "type": "palette",
                "source": "end4",
                "shellDark": dark,
                "dark": roles(&d),
                "light": roles(&l),
            });
        }
    }
    let dark = match config.color_scheme {
        ColorScheme::Dark => true,
        ColorScheme::Light => false,
        ColorScheme::System => system_dark.unwrap_or(true),
    };
    json!({
        "type": "palette",
        "source": "seed",
        "shellDark": dark,
        "dark": roles(&M3Palette::from_seed(&config.seed_color, true)),
        "light": roles(&M3Palette::from_seed(&config.seed_color, false)),
    })
}

/// The end-4 wallpaper scaled down and encoded as a JPEG data URL that fits
/// in one native message.
pub fn wallpaper_message(max_width: u32) -> Value {
    let data_url = sources::end4_wallpaper_path().and_then(|path| encode_wallpaper(&path, max_width));
    json!({"type": "wallpaper", "dataUrl": data_url})
}

fn encode_wallpaper(path: &std::path::Path, max_width: u32) -> Option<String> {
    use base64::Engine as _;
    let img = image::open(path).ok()?;
    let mut width = max_width.clamp(320, 3840);
    loop {
        let scaled = if img.width() > width {
            img.resize(width, u32::MAX, image::imageops::FilterType::Triangle)
        } else {
            img.clone()
        };
        let mut buf = Vec::new();
        let rgb = image::DynamicImage::ImageRgb8(scaled.to_rgb8());
        rgb.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg).ok()?;
        let url = format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(&buf));
        // Leave room for the JSON wrapper.
        if url.len() < MAX_OUTGOING - 1024 || width <= 320 {
            return (url.len() < MAX_OUTGOING - 1024).then_some(url);
        }
        width = width * 3 / 4;
    }
}

enum Event {
    Incoming(Value),
    PaletteChanged,
    Closed,
}

/// Run the host until the browser closes stdin.
pub fn run() -> anyhow::Result<()> {
    let (tx, rx) = flume::unbounded();

    let stdin_tx = tx.clone();
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        loop {
            match read_message(&mut stdin) {
                Ok(Some(v)) => {
                    if stdin_tx.send(Event::Incoming(v)).is_err() {
                        break;
                    }
                }
                _ => {
                    let _ = stdin_tx.send(Event::Closed);
                    break;
                }
            }
        }
    });

    // Watch end-4's generated colors and our own config.
    let watch_tx = tx.clone();
    let _watcher = {
        use notify::Watcher;
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                if ev.kind.is_create() || ev.kind.is_modify() {
                    let relevant = ev.paths.iter().any(|p| {
                        p.file_name().is_some_and(|n| n == "colors.json" || n == "config.toml")
                    });
                    if relevant {
                        let _ = watch_tx.send(Event::PaletteChanged);
                    }
                }
            }
        })
        .ok()
        .map(|mut w| {
            for dir in [
                sources::end4_colors_path().and_then(|p| p.parent().map(PathBuf::from)),
                Config::path().and_then(|p| p.parent().map(PathBuf::from)),
            ]
            .into_iter()
            .flatten()
            {
                if dir.is_dir() {
                    let _ = w.watch(&dir, notify::RecursiveMode::NonRecursive);
                }
            }
            w
        })
    };

    let mut stdout = std::io::stdout().lock();
    let mut last_palette = Value::Null;
    let send_palette = |out: &mut std::io::StdoutLock, last: &mut Value, force: bool| -> std::io::Result<()> {
        let msg = palette_message(&Config::load(), sources::end4_colors_path(), None);
        if force || msg != *last {
            write_message(out, &msg)?;
            *last = msg;
        }
        Ok(())
    };

    while let Ok(ev) = rx.recv() {
        match ev {
            Event::Closed => break,
            Event::PaletteChanged => {
                // matugen writes several files in a burst; let it settle.
                std::thread::sleep(Duration::from_millis(150));
                while let Ok(Event::PaletteChanged) = rx.try_recv() {}
                send_palette(&mut stdout, &mut last_palette, false)?;
            }
            Event::Incoming(msg) => match msg.get("type").and_then(Value::as_str) {
                Some("hello") | Some("palette") => send_palette(&mut stdout, &mut last_palette, true)?,
                Some("wallpaper") => {
                    let w = msg.get("maxWidth").and_then(Value::as_u64).unwrap_or(1920) as u32;
                    write_message(&mut stdout, &wallpaper_message(w))?;
                }
                _ => write_message(&mut stdout, &json!({"type": "error", "error": "unknown message"}))?,
            },
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_round_trip() {
        let mut buf = Vec::new();
        write_message(&mut buf, &json!({"type": "hello", "n": 1})).unwrap();
        write_message(&mut buf, &json!({"type": "x"})).unwrap();
        assert_eq!(&buf[..4], &(br#"{"type":"hello","n":1}"#.len() as u32).to_le_bytes());
        let mut r = std::io::Cursor::new(buf);
        assert_eq!(read_message(&mut r).unwrap(), Some(json!({"type": "hello", "n": 1})));
        assert_eq!(read_message(&mut r).unwrap(), Some(json!({"type": "x"})));
        assert_eq!(read_message(&mut r).unwrap(), None);
    }

    #[test]
    fn bad_frames_are_errors() {
        let mut r = std::io::Cursor::new(vec![3, 0, 0, 0, b'{', b'x', b'}']);
        assert!(read_message(&mut r).is_err());
        let huge = serde_json::Value::String("a".repeat(MAX_OUTGOING));
        assert!(write_message(&mut Vec::new(), &huge).is_err());
    }

    #[test]
    fn palette_from_end4_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("colors.json");
        std::fs::write(&file, r##"{"primary": "#ff0000", "background": "#101010", "surface": "#111111"}"##).unwrap();
        let msg = palette_message(&Config::default(), Some(file), None);
        assert_eq!(msg["source"], "end4");
        assert_eq!(msg["shellDark"], true);
        assert_eq!(msg["dark"]["primary"], "#FF0000");
        assert_eq!(msg["dark"]["surface"], "#111111");
        assert!(msg["light"]["surface"].as_str().unwrap().starts_with('#'));
        assert_eq!(msg["light"].as_object().unwrap().len(), msg["dark"].as_object().unwrap().len());
    }

    #[test]
    fn palette_from_seed() {
        let c = Config { color_scheme: ColorScheme::Light, ..Config::default() };
        let msg = palette_message(&c, None, None);
        assert_eq!(msg["source"], "seed");
        assert_eq!(msg["shellDark"], false);
        assert!(msg["dark"]["on_surface"].is_string());
    }
}
