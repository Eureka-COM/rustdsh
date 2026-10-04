use std::io::Read;

/// Heuristic token estimator: ~4 chars/token for mixed text.
/// CJK chars count ~1 token each. O(n), std-only.
pub fn estimate_tokens(s: &str) -> usize {
    let mut tokens = 0usize;
    let mut ascii_run = 0usize;
    for ch in s.chars() {
        if ch.is_ascii() {
            ascii_run += 1;
            if ascii_run == 4 {
                tokens += 1;
                ascii_run = 0;
            }
        } else {
            if ascii_run > 0 {
                tokens += ascii_run.div_ceil(4);
                ascii_run = 0;
            }
            tokens += 1;
        }
    }
    tokens + ascii_run.div_ceil(4)
}

/// Keep head+tail within budget; middle replaced with marker.
pub fn prune_to_budget(s: &str, max_tokens: usize) -> String {
    if estimate_tokens(s) <= max_tokens {
        return s.to_string();
    }
    let target_chars = max_tokens.saturating_mul(4).max(256);
    let head_chars = target_chars * 2 / 3;
    let tail_chars = target_chars - head_chars;
    let head: String = s.chars().take(head_chars).collect();
    let tail: String = s.chars().rev().take(tail_chars).collect::<String>().chars().rev().collect();
    format!("{head}\n\n...[rdsh pruned {}->{} tokens]...\n\n{tail}", estimate_tokens(s), max_tokens)
}

fn read_stdin() -> anyhow::Result<String> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

pub fn cmd_tokens(files: Vec<String>, preview: usize) -> anyhow::Result<()> {
    if files.is_empty() {
        let text = read_stdin()?;
        println!("{{\"tokens\": {}, \"chars\": {}}}", estimate_tokens(&text), text.len());
        return Ok(());
    }
    let mut total = 0usize;
    for f in &files {
        let text = std::fs::read_to_string(f)?;
        let t = estimate_tokens(&text);
        total += t;
        println!("{t:>8}  {f}");
        if preview > 0 {
            let pv: String = text.chars().take(preview).collect();
            println!("  preview: {}", pv.replace('\n', "\\n"));
        }
    }
    if files.len() > 1 {
        println!("{total:>8}  (total)");
    }
    Ok(())
}

pub fn cmd_prune(max_tokens: usize, file: Option<String>) -> anyhow::Result<()> {
    let text = match file {
        Some(f) => std::fs::read_to_string(&f)?,
        None => read_stdin()?,
    };
    let raw = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(v) => match v.get("content").and_then(|c| c.as_str()) {
            Some(c) if !c.is_empty() => c.to_string(),
            _ => text.clone(),
        },
        Err(_) => text.clone(),
    };
    let before = estimate_tokens(&raw);
    let pruned = prune_to_budget(&raw, max_tokens);
    let after = estimate_tokens(&pruned);
    eprintln!("[rdsh] tokens {before} -> {after} (budget {max_tokens})");
    println!("{pruned}");
    Ok(())
}
