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
            let ms = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            (ms, format_time(t))
        }
        Err(_) => (0, "-".to_string()),
    }
}

// Civil date from epoch secs (Howard Hinnant algorithm), UTC. No chrono needed.
fn format_time(t: std::time::SystemTime) -> String {
    let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
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
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", tod / 3600, (tod % 3600) / 60)
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
    for proj in projs {
        let pdir = std::path::Path::new(&root).join(&proj);
        let entries = match std::fs::read_dir(&pdir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for e in entries.filter_map(|e| e.ok()) {
            if !e.path().is_dir() {
                continue;
            }
            let id = e.file_name().to_string_lossy().into_owned();
            let (bytes, mtime, mtime_s) = dir_size_mtime(&e.path());
            out.push(Session { project: proj.clone(), id, bytes, mtime, mtime_s });
        }
    }
    out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    let total = out.len();
    let zstd = tokens && zstd_available();
    if tokens && !zstd {
        eprintln!("[rdsh] note: zstd CLI not found; token column shows stored-bytes/4 estimate");
    }
    for s in out.iter().take(limit) {
        let tok = if tokens {
            if zstd {
                match decompressed_bytes(&root, &s.project, &s.id) {
                    Some(b) => format!("~{}tok", b / 4),
                    None => format!("~{}tok?", s.bytes / 4),
                }
            } else {
                format!("~{}tok?", s.bytes / 4)
            }
        } else {
            "-".to_string()
        };
        println!("{:>8} {:>10} {} {}/{}", human_bytes(s.bytes), tok, s.mtime_s, s.project, s.id);
    }
    eprintln!("[rdsh] {total} session(s), showing up to {limit}");
    Ok(())
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

fn zstd_available() -> bool {
    std::process::Command::new("zstd").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
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
        let out = std::process::Command::new("zstd").arg("-dc").arg("--").arg(&p).output().ok()?;
        if out.status.success() {
            total += out.stdout.len() as u64;
            any = true;
        }
    }
    if any { Some(total) } else { None }
}

pub fn cmd_logs(tail: usize, grep: Option<String>, file: Option<String>) -> anyhow::Result<()> {
    let dir = format!("{}/logs", dsh_home());
    let path = match file {
        Some(f) => std::path::PathBuf::from(if f.contains('/') { f } else { format!("{dir}/{f}") }),
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
        lines = lines.into_iter().filter(|l| l.contains(pat)).collect();
    }
    let n = lines.len();
    let start = n.saturating_sub(tail);
    for l in &lines[start..] {
        println!("{l}");
    }
    eprintln!("[rdsh] {} line(s){}", n, grep.map(|g| format!(" matching {g:?}")).unwrap_or_default());
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
