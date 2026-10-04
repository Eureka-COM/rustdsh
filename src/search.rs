// Native recursive grep: std-only, skips hidden/vendor dirs, caps output.
// Two phases: sequential walk (fixed order) then parallel grep over files.
// Stdout matches a sequential scan: per-file hits merge in walk order.

pub fn cmd_search(pattern: &str, dir: &str, max: usize) -> anyhow::Result<()> {
    let mut files: Vec<std::path::PathBuf> = vec![];
    collect_files(std::path::Path::new(dir), &mut files);
    let nfiles = files.len();
    let per_file: Vec<Vec<String>> = if nfiles >= 32 {
        grep_parallel(pattern, &files)
    } else {
        files.iter().map(|p| grep_one(pattern, p)).collect()
    };
    // One locked, buffered stdout for the whole dump (same bytes out).
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let mut shown = 0usize;
    let mut outer_done = false;
    for fh in &per_file {
        if outer_done {
            break;
        }
        for line in fh {
            if shown >= max {
                outer_done = true;
                break;
            }
            let _ = writeln!(out, "{line}");
            shown += 1;
        }
    }
    let _ = out.flush();
    eprintln!("[rdsh] {shown} hit(s) in {nfiles} file(s)");
    Ok(())
}
const SKIP: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".venv",
    "dist",
    "build",
    ".next",
];

fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for e in entries.filter_map(|e| e.ok()) {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if SKIP.contains(&name.as_str()) || name.starts_with(".") {
                continue;
            }
            collect_files(&p, out);
        } else if p.is_file() {
            // No per-file stat here: the 2MB cap is enforced by the bounded
            // read in grep_one, saving one syscall per file. The stderr file
            // count therefore includes skipped oversized files (stdout hits
            // are unchanged).
            out.push(p);
        }
    }
}

/// Read at most 2MB+1 bytes as UTF-8. Returns None for missing files,
/// files over 2MB, and non-UTF-8 content — the same skip set as the old
/// metadata-check plus read_to_string combination (verified by diff).
fn read_capped(path: &std::path::Path) -> Option<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    f.take(2_000_001).read_to_end(&mut buf).ok()?;
    if buf.len() > 2_000_000 {
        return None;
    }
    String::from_utf8(buf).ok()
}

fn grep_one(pattern: &str, path: &std::path::Path) -> Vec<String> {
    let mut hits = vec![];
    if let Some(text) = read_capped(path) {
        for (i, line) in text.lines().enumerate() {
            if line.contains(pattern) {
                hits.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    i + 1,
                    truncate(line, 240)
                ));
            }
        }
    }
    hits
}

fn grep_parallel(pattern: &str, files: &[std::path::PathBuf]) -> Vec<Vec<String>> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 8);
    if threads <= 1 {
        return files.iter().map(|p| grep_one(pattern, p)).collect();
    }
    let chunk = files.len().div_ceil(threads);
    let mut out: Vec<Vec<Vec<String>>> = vec![];
    std::thread::scope(|s| {
        let mut handles = vec![];
        for c in files.chunks(chunk) {
            handles
                .push(s.spawn(move || c.iter().map(|p| grep_one(pattern, p)).collect::<Vec<_>>()));
        }
        for h in handles {
            out.push(h.join().unwrap_or_default());
        }
    });
    out.into_iter().flatten().collect()
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        return s.to_string();
    }
    let mut t: String = s.chars().take(n).collect();
    t.push(gt_sign());
    t
}

fn gt_sign() -> char {
    62 as char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capped_boundary() {
        let dir = std::env::temp_dir();
        let exact = dir.join("rdsh-cap-exact.txt");
        let over = dir.join("rdsh-cap-over.txt");
        let bin = dir.join("rdsh-cap-bin.bin");
        std::fs::write(&exact, vec![120u8; 2_000_000]).unwrap();
        let mut big = vec![120u8; 2_000_001];
        big.push(121);
        std::fs::write(&over, big).unwrap();
        std::fs::write(&bin, [0u8, 159, 146, 150]).unwrap();
        assert!(read_capped(&exact).is_some());
        assert!(read_capped(&over).is_none());
        assert!(read_capped(&bin).is_none());
        assert!(read_capped(&dir.join("rdsh-cap-missing.txt")).is_none());
        let _ = std::fs::remove_file(&exact);
        let _ = std::fs::remove_file(&over);
        let _ = std::fs::remove_file(&bin);
    }
}
