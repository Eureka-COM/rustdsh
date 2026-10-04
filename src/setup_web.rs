//! `rdsh setup --web`: floating first-run connect UI on localhost.
//! One-shot local server (127.0.0.1 only): Apple-style glass page showing
//! OAuth/API-key status. Writes only on explicit key submit (allowlisted
//! ref names); browser auto-opens on interactive terminals.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const HTML: &str = include_str!("setup.html");
const ICON: &str = include_str!("../assets/icon.svg");

pub fn cmd_setup_web(port: u16) -> anyhow::Result<()> {
    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .map_err(|e| anyhow::anyhow!("cannot listen on 127.0.0.1:{port}: {e}"))?;
    let url = format!("http://{}/", listener.local_addr()?);
    eprintln!("[rdsh setup] floating UI: {url}  (localhost only, Ctrl-C to stop)");
    use std::io::IsTerminal as _;
    if std::io::stdin().is_terminal() {
        if let Err(e) = crate::auth::open_browser(&url) {
            eprintln!("[rdsh setup] could not open browser ({e:#}); open the URL manually");
        }
    }
    listener.set_nonblocking(true)?;
    let done = Arc::new(AtomicBool::new(false));
    loop {
        if done.load(Ordering::Relaxed) {
            break;
        }
        match listener.accept() {
            Ok((s, _)) => {
                let d = done.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle(s, &d) {
                        eprintln!("[rdsh setup] {e:#}");
                    }
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("[rdsh setup] accept: {e:#}");
                break;
            }
        }
    }
    Ok(())
}

fn handle(mut s: std::net::TcpStream, done: &AtomicBool) -> anyhow::Result<()> {
    use std::borrow::Cow;
    use std::io::{Read, Write};
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let mut buf = [0u8; 65536];
    let n = s.read(&mut buf)?;
    let req_cow = String::from_utf8_lossy(&buf[..n]);
    let req: &str = &req_cow;
    let mut lines = req.lines();
    let head = lines.next().unwrap_or("");
    let mut parts = head.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let body: &str = match req.find("\r\n\r\n") {
        Some(i) => &req[i + 4..],
        None => "",
    };
    let (status, ctype, payload): (u16, &str, Cow<str>) = match (method, path) {
        ("GET", "/") => (200, "text/html; charset=utf-8", Cow::Borrowed(HTML)),
        ("GET", "/icon.svg") => (200, "image/svg+xml", Cow::Borrowed(ICON)),
        ("GET", "/api/status") => (
            200,
            "application/json",
            Cow::Owned(crate::auth::setup_status_json()),
        ),
        ("POST", "/api/key") => match store_key_body(body) {
            Ok(stored) => (
                200,
                "application/json",
                Cow::Owned(serde_json::json!({"stored": stored}).to_string()),
            ),
            Err(e) => (
                400,
                "application/json",
                Cow::Owned(serde_json::json!({"error": e.to_string()}).to_string()),
            ),
        },
        ("POST", "/api/done") => {
            done.store(true, Ordering::Relaxed);
            (200, "application/json", Cow::Borrowed("{\"ok\":true}"))
        }
        _ => (
            404,
            "application/json",
            Cow::Borrowed("{\"error\":\"not found\"}"),
        ),
    };
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!("HTTP/1.1 {status} {status_text}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", payload.len());
    s.write_all(head.as_bytes())?;
    s.write_all(payload.as_bytes())?;
    Ok(())
}

fn store_key_body(body: &str) -> anyhow::Result<bool> {
    let v: serde_json::Value = serde_json::from_str(body)?;
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
    let value = v.get("value").and_then(|x| x.as_str()).unwrap_or("");
    crate::auth::setup_store_key(name, value)
}
