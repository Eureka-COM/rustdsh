//! `rdsh serve`: std-only local dashboard (127.0.0.1 only, no extra deps).
//! Serves the embedded UI plus a tiny JSON API. Read-only operations only:
//! no boot, no file writes, no command execution from HTTP.

const UI: &str = include_str!("ui.html");

pub fn cmd_serve(port: u16) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = std::net::TcpListener::bind(&addr).map_err(|e| {
        anyhow::anyhow!("cannot listen on {addr}: {e} (dsh web GUI also uses 3080; try --port 38080)")
    })?;
    eprintln!("[rdsh] dashboard: http://{addr}/  (Ctrl-C to stop, localhost only)");
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(move || {
                    if let Err(e) = handle(s) {
                        eprintln!("[rdsh serve] {e:#}");
                    }
                });
            }
            Err(e) => eprintln!("[rdsh serve] accept: {e}"),
        }
    }
    Ok(())
}

fn handle(mut s: std::net::TcpStream) -> anyhow::Result<()> {
    use std::io::{Read, Write};
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let mut buf = vec![0u8; 65536];
    let n = s.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]).into_owned();
    let mut lines = req.lines();
    let head = lines.next().unwrap_or("");
    let mut parts = head.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let (path, query) = match target.find('?') {
        Some(i) => (&target[..i], &target[i + 1..]),
        None => (target, ""),
    };
    let body = match req.find("\r\n\r\n") {
        Some(i) => req[i + 4..].to_string(),
        None => String::new(),
    };
    let (status, ctype, payload): (u16, &str, String) = match (method, path) {
        ("GET", "/") => (200, "text/html; charset=utf-8", UI.to_string()),
        ("GET", "/api/version") => (200, "application/json", serde_json::json!({"name": "rdsh", "version": env!("CARGO_PKG_VERSION")}).to_string()),
        ("GET", "/api/doctor") => (200, "application/json", doctor_json()),
        ("POST", "/api/tokens") => {
            let text = serde_json::from_str::<serde_json::Value>(&body)
                .ok().and_then(|v| v.get("text").and_then(|t| t.as_str()).map(|t| t.to_string()))
                .unwrap_or_default();
            let t = crate::tokens::estimate_tokens(&text);
            (200, "application/json", serde_json::json!({"tokens": t, "chars": text.len()}).to_string())
        }
        ("POST", "/api/prune") => {
            let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
            let text = v.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
            let max = v.get("max_tokens").and_then(|m| m.as_u64()).unwrap_or(4000) as usize;
            let max = max.clamp(100, 200_000);
            let before = crate::tokens::estimate_tokens(&text);
            let pruned = crate::tokens::prune_to_budget(&text, max);
            let after = crate::tokens::estimate_tokens(&pruned);
            (200, "application/json", serde_json::json!({"pruned": pruned, "before": before, "after": after, "budget": max}).to_string())
        }
        ("GET", "/api/bench") => (200, "application/json", bench_json(query)),
        ("GET", "/api/sessions") => {
            let n: usize = query
                .split("&")
                .find_map(|kv| {
                    let mut it = kv.splitn(2, "=");
                    match (it.next(), it.next()) {
                        (Some("limit"), Some(v)) => v.parse().ok(),
                        _ => None,
                    }
                })
                .unwrap_or(20)
                .clamp(1, 100);
            (200, "application/json", crate::inspect::sessions_json(n))
        }
        ("GET", "/api/skills") => (200, "application/json", crate::inspect::names_json("skills")),
        ("GET", "/api/profiles") => (200, "application/json", crate::inspect::names_json("profiles")),
        _ => (404, "application/json", serde_json::json!({"error": "not found"}).to_string()),
    };
    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!("HTTP/1.1 {status} {status_text}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", payload.len());
    s.write_all(head.as_bytes())?;
    s.write_all(payload.as_bytes())?;
    s.flush()?;
    Ok(())
}

fn doctor_json() -> String {
    let orig = crate::passthrough::find_original_dsh();
    let home = std::env::var("DSH_HOME").unwrap_or_else(|_| {
        let h = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{h}/.dsh")
    });
    let profiles = std::fs::read_dir(format!("{home}/profiles")).map(|d| d.count()).unwrap_or(0);
    serde_json::json!({
        "original_dsh": orig,
        "dsh_home": home,
        "local_profiles": profiles,
        "slim": crate::slim::describe(),
        "version": env!("CARGO_PKG_VERSION"),
    })
    .to_string()
}

fn bench_json(query: &str) -> String {
    let n: u32 = query.split('&').find_map(|kv| {
        let mut it = kv.splitn(2, '=');
        match (it.next(), it.next()) {
            (Some("n"), Some(v)) => v.parse().ok(),
            _ => None,
        }
    }).unwrap_or(3).clamp(1, 5);
    let me = std::env::current_exe().ok();
    let mut mine = vec![];
    if let Some(exe) = me {
        for _ in 0..n {
            let t = std::time::Instant::now();
            let ok = std::process::Command::new(&exe).arg("--version").status().map(|s| s.success()).unwrap_or(false);
            if !ok {
                break;
            }
            mine.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    serde_json::json!({"rdsh_version_ms": mine, "n": n}).to_string()
}
