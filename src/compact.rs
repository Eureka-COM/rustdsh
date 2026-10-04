use crate::tokens::{estimate_tokens, prune_to_budget};

/// Compact a JSONL session transcript: keep first (system) + last turns
/// within budget. Never deletes source; prints to stdout.
pub fn cmd_compact(file: &str, max_tokens: usize) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(file)?;
    let lines: Vec<&str> = text.lines().collect();
    let total: usize = lines.iter().map(|l| estimate_tokens(l)).sum();
    if total <= max_tokens {
        println!("{text}");
        eprintln!("[rdsh] no compaction needed ({total} <= {max_tokens} tokens)");
        return Ok(());
    }
    let first = lines.first().copied().unwrap_or("");
    let mut kept: Vec<&str> = vec![];
    let mut used = estimate_tokens(first) + 200;
    for l in lines.iter().rev() {
        let t = estimate_tokens(l);
        if used + t > max_tokens {
            break;
        }
        used += t;
        kept.push(l);
    }
    kept.reverse();
    let dropped = lines.len().saturating_sub(kept.len());
    let summary = format!("...[rdsh compact: dropped ~{dropped} turn(s), {total}-><={max_tokens} tokens]...");
    let mut out = String::new();
    out.push_str(first);
    out.push('\n');
    out.push_str(&summary);
    out.push('\n');
    for l in &kept {
        if *l == first {
            continue;
        }
        out.push_str(l);
        out.push('\n');
    }
    let out = prune_to_budget(&out, max_tokens);
    println!("{out}");
    eprintln!("[rdsh] compacted {total} -> ~{} tokens ({} lines kept of {})", estimate_tokens(&out), kept.len(), lines.len());
    Ok(())
}
