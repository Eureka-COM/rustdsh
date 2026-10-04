//! Read-only inspection of $DSH_HOME without Node: sessions, logs, skills, profiles.
//! Never writes; missing dirs are reported, not errors (exit 0 with a note).

pub fn dsh_home() -> String {
    std::env::var("DSH_HOME").unwrap_or_else(|_| {
        let h = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{h}/.dsh")
    })
}

fn mtime_ms(p: &std::path::Path) -> (u64, String) {
    match std::fs::metadata(p).and_then(|m| m.modified()) {
        Ok(t) => {
            let ms = t
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            (ms, format_time(t))
        }
        Err(_) => (0, "-".to_string()),
    }
}

// Civil date from epoch secs (Howard Hinnant algorithm), UTC. No chrono needed.
fn format_time(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        tod / 3600,
        (tod % 3600) / 60
    )
}

fn human_bytes(n: u64) -> String {
    const U: &[&str] = &["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < U.len() {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n}{}", U[u])
    } else {
        format!("{v:.1}{}", U[u])
    }
}

pub fn cmd_profiles() -> anyhow::Result<()> {
    list_dir(&format!("{}/profiles", dsh_home()), "profile")
}

pub fn cmd_skills() -> anyhow::Result<()> {
    list_dir(&format!("{}/skills", dsh_home()), "skill")
}

fn list_dir(dir: &str, kind: &str) -> anyhow::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            println!("# no {kind} dir at {dir}");
            return Ok(());
        }
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for n in &names {
        println!("{n}");
    }
    eprintln!("[rdsh] {} {kind}(s)", names.len());
    Ok(())
}

struct Session {
    project: String,
    id: String,
    bytes: u64,
    mtime: u64,
    mtime_s: String,
}

pub fn cmd_sessions(project: Option<String>, limit: usize, tokens: bool) -> anyhow::Result<()> {
    let root = format!("{}/sessions", dsh_home());
    let projs: Vec<String> = match &project {
        Some(p) => vec![p.clone()],
        None => match std::fs::read_dir(&root) {
            Ok(e) => e
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => {
                println!("# no sessions dir at {root}");
                return Ok(());
            }
        },
    };
    let mut out: Vec<Session> = vec![];
    if projs.len() >= 2 {
        std::thread::scope(|s| {
            let mut handles = vec![];
            for proj in &projs {
                let pdir = std::path::Path::new(&root).join(proj);
                let proj = proj.clone();
                handles.push(s.spawn(move || scan_project(&pdir, &proj)));
            }
            for h in handles {
                out.extend(h.join().unwrap_or_default());
            }
        });
    } else {
        for proj in &projs {
            let pdir = std::path::Path::new(&root).join(proj);
            out.extend(scan_project(&pdir, proj));
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.mtime));
    let total = out.len();
    let zstd = tokens && zstd_available();
    if tokens && !zstd {
        eprintln!("[rdsh] note: zstd CLI not found; token column shows stored-bytes/4 estimate");
    }
    let shown: Vec<&Session> = out.iter().take(limit).collect();
    let sizes: Vec<Option<u64>> = if tokens && zstd && shown.len() >= 2 {
        batch_decompressed(&root, &shown)
    } else {
        shown
            .iter()
            .map(|s| decompressed_bytes(&root, &s.project, &s.id))
            .collect()
    };
    for (s, decomp) in shown.iter().zip(sizes.iter()) {
        let tok = if tokens {
            if zstd {
                match decomp {
                    Some(b) => format!("~{}tok", b / 4),
                    None => format!("~{}tok?", s.bytes / 4),
                }
            } else {
                format!("~{}tok?", s.bytes / 4)
            }
        } else {
            "-".to_string()
        };
        println!(
            "{:>8} {:>10} {} {}/{}",
            human_bytes(s.bytes),
            tok,
            s.mtime_s,
            s.project,
            s.id
        );
    }
    eprintln!("[rdsh] {total} session(s), showing up to {limit}");
    Ok(())
}

fn scan_project(pdir: &std::path::Path, proj: &str) -> Vec<Session> {
    let mut v = vec![];
    let entries = match std::fs::read_dir(pdir) {
        Ok(e) => e,
        Err(_) => return v,
    };
    for e in entries.filter_map(|e| e.ok()) {
        if !e.path().is_dir() {
            continue;
        }
        let id = e.file_name().to_string_lossy().into_owned();
        let (bytes, mtime, mtime_s) = dir_size_mtime(&e.path());
        v.push(Session {
            project: proj.to_string(),
            id,
            bytes,
            mtime,
            mtime_s,
        });
    }
    v
}

fn dir_size_mtime(dir: &std::path::Path) -> (u64, u64, String) {
    let mut bytes = 0u64;
    let mut mtime = 0u64;
    let mut mtime_s = "-".to_string();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_file() {
                if let Ok(m) = e.metadata() {
                    bytes += m.len();
                }
                let (m, ms) = mtime_ms(&p);
                if m > mtime {
                    mtime = m;
                    mtime_s = ms;
                }
            }
        }
    }
    (bytes, mtime, mtime_s)
}

fn batch_decompressed(root: &str, shown: &[&Session]) -> Vec<Option<u64>> {
    let mut out: Vec<Option<u64>> = vec![None; shown.len()];
    std::thread::scope(|s| {
        let mut handles = vec![];
        for sess in shown {
            let root = root.to_string();
            let proj = sess.project.clone();
            let id = sess.id.clone();
            handles.push(s.spawn(move || decompressed_bytes(&root, &proj, &id)));
        }
        for (i, h) in handles.into_iter().enumerate() {
            out[i] = h.join().unwrap_or(None);
        }
    });
    out
}

fn zstd_available() -> bool {
    std::process::Command::new("zstd")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn decompressed_bytes(root: &str, project: &str, id: &str) -> Option<u64> {
    let dir = std::path::Path::new(root).join(project).join(id);
    let entries = std::fs::read_dir(dir).ok()?;
    let mut total = 0u64;
    let mut any = false;
    for e in entries.filter_map(|e| e.ok()) {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().into_owned();
        if !name.ends_with(".zstd") {
            continue;
        }
        let out = std::process::Command::new("zstd")
            .arg("-dc")
            .arg("--")
            .arg(&p)
            .output()
            .ok()?;
        if out.status.success() {
            total += out.stdout.len() as u64;
            any = true;
        }
    }
    if any {
        Some(total)
    } else {
        None
    }
}

pub fn cmd_logs(tail: usize, grep: Option<String>, file: Option<String>) -> anyhow::Result<()> {
    let dir = format!("{}/logs", dsh_home());
    let path = match file {
        Some(f) => std::path::PathBuf::from(if f.contains('/') {
            f
        } else {
            format!("{dir}/{f}")
        }),
        None => match latest_file(&dir) {
            Some(p) => p,
            None => {
                println!("# no logs at {dir}");
                return Ok(());
            }
        },
    };
    eprintln!("[rdsh] reading {}", path.display());
    let text = std::fs::read_to_string(&path)?;
    let mut lines: Vec<&str> = text.lines().collect();
    if let Some(pat) = grep.as_deref() {
        lines.retain(|l| l.contains(pat));
    }
    let n = lines.len();
    let start = n.saturating_sub(tail);
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for l in &lines[start..] {
        let _ = writeln!(out, "{l}");
    }
    let _ = out.flush();
    eprintln!(
        "[rdsh] {} line(s){}",
        n,
        grep.map(|g| format!(" matching {g:?}")).unwrap_or_default()
    );
    Ok(())
}

fn latest_file(dir: &str) -> Option<std::path::PathBuf> {
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    for e in std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
        if !e.path().is_file() {
            continue;
        }
        let (m, _) = mtime_ms(&e.path());
        if best.as_ref().map(|(bm, _)| m > *bm).unwrap_or(true) {
            best = Some((m, e.path()));
        }
    }
    best.map(|(_, p)| p)
}

fn mtime_of(v: &serde_json::Value) -> &str {
    v.get("mtime").and_then(|m| m.as_str()).unwrap_or("")
}

fn dir_names(dir: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|e| {
            e.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

pub fn names_json(kind: &str) -> String {
    let dir = format!("{}/{}", dsh_home(), kind);
    serde_json::json!({"kind": kind, "names": dir_names(&dir)}).to_string()
}

pub fn sessions_json(limit: usize) -> String {
    let root = format!("{}/sessions", dsh_home());
    let mut out: Vec<serde_json::Value> = vec![];
    let projs: Vec<String> = std::fs::read_dir(&root)
        .map(|e| {
            e.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    for proj in &projs {
        let pdir = std::path::Path::new(&root).join(proj);
        if let Ok(entries) = std::fs::read_dir(&pdir) {
            for e in entries.filter_map(|e| e.ok()) {
                if !e.path().is_dir() {
                    continue;
                }
                let (bytes, _m, mtime_s) = dir_size_mtime(&e.path());
                out.push(serde_json::json!({
                    "project": proj,
                    "id": e.file_name().to_string_lossy(),
                    "bytes": bytes,
                    "mtime": mtime_s,
                }));
            }
        }
    }
    out.sort_by(|a, b| mtime_of(b).cmp(mtime_of(a)));
    out.truncate(limit.clamp(1, 100));
    serde_json::json!({"sessions": out, "projects": projs.len()}).to_string()
}
