/// Native recursive grep: std-only, skips hidden/vendor dirs, caps output.
pub fn cmd_search(pattern: &str, dir: &str, max: usize) -> anyhow::Result<()> {
    let mut hits = 0usize;
    let mut files = 0usize;
    search_dir(std::path::Path::new(dir), pattern, max, &mut hits, &mut files)?;
    eprintln!("[rdsh] {hits} hit(s) in {files} file(s)");
    Ok(())
}

const SKIP: &[&str] = &[".git", "node_modules", "target", ".venv", "dist", "build", ".next"];

fn search_dir(dir: &std::path::Path, pat: &str, max: usize, hits: &mut usize, files: &mut usize) -> anyhow::Result<()> {
    if *hits >= max {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for e in entries.filter_map(|e| e.ok()) {
        if *hits >= max {
            break;
        }
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if SKIP.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            search_dir(&p, pat, max, hits, files)?;
        } else if p.is_file() {
            if let Ok(md) = e.metadata() {
                if md.len() > 2_000_000 {
                    continue;
                }
            }
            *files += 1;
            if let Ok(text) = std::fs::read_to_string(&p) {
                for (i, line) in text.lines().enumerate() {
                    if line.contains(pat) {
                        println!("{}:{}: {}", p.display(), i + 1, truncate(line, 240));
                        *hits += 1;
                        if *hits >= max {
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        return s.to_string();
    }
    let mut t: String = s.chars().take(n).collect();
    t.push('>');
    t
}
