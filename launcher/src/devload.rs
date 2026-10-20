//! "Development load": install the (unsigned) extension temporarily at each
//! start, the way `web-ext run` does, through the Firefox remote debugging
//! protocol. Only used when the user opts in. The debugging server listens
//! on a Unix socket in a private runtime folder rather than a TCP port.

use crate::profile::EXTENSION_ID;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// `$XDG_RUNTIME_DIR/m3-browser/devtools.sock`, in a 0700 folder.
pub fn socket_path() -> PathBuf {
    let base = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
    base.join("m3-browser").join("devtools.sock")
}

pub fn prepare_socket_dir() -> std::io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = socket_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

/// RDP packets are `<decimal length>:<json>`.
pub fn encode(msg: &Value) -> Vec<u8> {
    let body = msg.to_string();
    format!("{}:{}", body.len(), body).into_bytes()
}

pub fn decode(r: &mut impl BufRead) -> std::io::Result<Value> {
    let mut len = Vec::new();
    r.read_until(b':', &mut len)?;
    len.pop();
    let len: usize = std::str::from_utf8(&len)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad packet length"))?;
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Client {
    fn connect(path: &Path) -> std::io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(Duration::from_secs(20)))?;
        let mut c = Client { reader: BufReader::new(stream.try_clone()?), writer: stream };
        // The server greets first.
        c.recv()?;
        Ok(c)
    }

    fn recv(&mut self) -> std::io::Result<Value> {
        decode(&mut self.reader)
    }

    /// Send a request and wait for the reply from the same actor (skipping
    /// unrelated notifications).
    fn request(&mut self, msg: Value) -> std::io::Result<Value> {
        let to = msg["to"].as_str().unwrap_or_default().to_string();
        self.writer.write_all(&encode(&msg))?;
        loop {
            let reply = self.recv()?;
            if reply["from"].as_str() == Some(to.as_str()) {
                return Ok(reply);
            }
        }
    }
}

/// Wait for the browser's debugging socket, then install the extension
/// unless it's already loaded in this browser session.
pub fn load_when_ready(socket: &Path, extension: &Path, timeout: Duration) -> Result<String, String> {
    let start = Instant::now();
    let mut client = loop {
        match Client::connect(socket) {
            Ok(c) => break c,
            Err(e) if start.elapsed() > timeout => {
                return Err(format!("The browser's debugging socket didn't come up: {e}"));
            }
            Err(_) => std::thread::sleep(Duration::from_millis(400)),
        }
    };
    let err = |e: std::io::Error| e.to_string();
    if let Ok(list) = client.request(json!({"to": "root", "type": "listAddons"})) {
        if list["addons"].as_array().is_some_and(|a| a.iter().any(|x| x["id"] == EXTENSION_ID)) {
            return Ok("already loaded".into());
        }
    }
    let root = client.request(json!({"to": "root", "type": "getRoot"})).map_err(err)?;
    let actor = root["addonsActor"]
        .as_str()
        .ok_or("The browser doesn't offer add-on installation over the debugging protocol.")?
        .to_string();
    let reply = client
        .request(json!({
            "to": actor,
            "type": "installTemporaryAddon",
            "addonPath": extension.to_string_lossy(),
            "openDevTools": false,
        }))
        .map_err(err)?;
    if let Some(e) = reply.get("error") {
        return Err(format!("{e}: {}", reply["message"].as_str().unwrap_or_default()));
    }
    Ok(reply["addon"]["id"].as_str().unwrap_or(EXTENSION_ID).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_round_trip() {
        let mut buf = encode(&json!({"to": "root", "type": "getRoot"}));
        buf.extend(encode(&json!({"from": "root", "ok": "ü"})));
        let mut r = std::io::Cursor::new(buf);
        assert_eq!(decode(&mut r).unwrap()["type"], "getRoot");
        assert_eq!(decode(&mut r).unwrap()["ok"], "ü");
        assert!(decode(&mut std::io::Cursor::new(b"x:{}".to_vec())).is_err());
    }

    /// A fake debugger server on a Unix socket, speaking just enough RDP.
    #[test]
    fn installs_through_a_fake_server() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("rdp.sock");
        let listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut w = stream.try_clone().unwrap();
            let mut r = BufReader::new(stream);
            w.write_all(&encode(&json!({"from": "root", "applicationType": "browser"}))).unwrap();
            let mut seen = Vec::new();
            loop {
                let Ok(msg) = decode(&mut r) else { break };
                seen.push(msg["type"].as_str().unwrap_or_default().to_string());
                let reply = match msg["type"].as_str() {
                    Some("listAddons") => json!({"from": "root", "addons": []}),
                    Some("getRoot") => json!({"from": "root", "addonsActor": "server1.addons"}),
                    Some("installTemporaryAddon") => {
                        // An unrelated notification first, which the client must skip.
                        w.write_all(&encode(&json!({"from": "root", "type": "tabListChanged"}))).unwrap();
                        json!({"from": "server1.addons", "addon": {"id": EXTENSION_ID}})
                    }
                    _ => json!({"from": "root", "error": "unknown"}),
                };
                w.write_all(&encode(&reply)).unwrap();
            }
            seen
        });
        let id = load_when_ready(&sock, Path::new("/tmp/ext"), Duration::from_secs(5)).unwrap();
        assert_eq!(id, EXTENSION_ID);
        drop(id);
        let seen = server.join().unwrap();
        assert_eq!(seen, vec!["listAddons", "getRoot", "installTemporaryAddon"]);
    }
}
