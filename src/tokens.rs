use std::io::{Read, Write};

/// Heuristic token estimator: ~4 chars/token for mixed text.
/// CJK chars count ~1 token each. O(n), std-only.
pub fn estimate_tokens(s: &str) -> usize {
    if s.is_ascii() {
        return s.len().div_ceil(4);
    }
    estimate_bytes(s.as_bytes())
}

/// Byte-level twin of the char rule above: ASCII bytes accumulate in runs
/// of 4 -> 1 token, each non-ASCII scalar (exactly one UTF-8 lead byte)
/// counts 1 token, continuation bytes count nothing. Same result as
/// iterating `chars()`, without UTF-8 decoding overhead.
fn estimate_bytes(b: &[u8]) -> usize {
    let mut tokens = 0usize;
    let mut ascii_run = 0usize;
    for &c in b {
        if c < 0x80 {
            ascii_run += 1;
            if ascii_run == 4 {
                tokens += 1;
                ascii_run = 0;
            }
        } else if c & 0xC0 != 0x80 {
            // UTF-8 lead byte: one non-ASCII char.
            if ascii_run > 0 {
                tokens += ascii_run.div_ceil(4);
                ascii_run = 0;
            }
            tokens += 1;
        }
    }
    tokens + ascii_run.div_ceil(4)
}

/// Byte offset just past the first `n` chars (`s.len()` when shorter).
/// Pure byte-class scan, no per-char allocation.
fn head_byte_end(s: &str, n: usize) -> usize {
    let mut chars = 0usize;
    for (i, &b) in s.as_bytes().iter().enumerate() {
        if b & 0xC0 != 0x80 {
            chars += 1;
            if chars > n {
                return i;
            }
        }
    }
    s.len()
}

/// Byte offset where the last `n` chars start (0 when longer).
fn tail_byte_start(s: &str, n: usize) -> usize {
    let mut chars = 0usize;
    for (i, &b) in s.as_bytes().iter().enumerate().rev() {
        if b & 0xC0 != 0x80 {
            chars += 1;
            if chars > n {
                return i + utf8_len(b);
            }
        }
    }
    0
}

/// Length in bytes of the scalar starting with lead byte `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b & 0xE0 == 0xC0 {
        2
    } else if b & 0xF0 == 0xE0 {
        3
    } else {
        4
    }
}

/// Keep head+tail within budget; middle replaced with marker.
pub fn prune_to_budget(s: &str, max_tokens: usize) -> String {
    prune_with_total(s, max_tokens, estimate_tokens(s))
}

/// Shared impl for callers that already estimated `s` (saves a scan).
fn prune_with_total(s: &str, max_tokens: usize, total: usize) -> String {
    if total <= max_tokens {
        return s.to_string();
    }
    let target_chars = max_tokens.saturating_mul(4).max(256);
    let head_chars = target_chars * 2 / 3;
    let tail_chars = target_chars - head_chars;
    // Slice at char boundaries instead of collecting chars (the old code
    // built a Vec<char> for the tail). Same head/tail chars, no middleman.
    let head = &s[..head_byte_end(s, head_chars)];
    let tail = if tail_chars == 0 {
        ""
    } else {
        &s[tail_byte_start(s, tail_chars)..]
    };
    format!(
        "{head}\n\n...[rdsh pruned {}->{} tokens]...\n\n{tail}",
        total, max_tokens
    )
}

fn read_stdin() -> anyhow::Result<String> {
    let mut buf = String::with_capacity(1 << 16);
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn stdout_writer() -> std::io::BufWriter<std::io::StdoutLock<'static>> {
    std::io::BufWriter::with_capacity(256 * 1024, std::io::stdout().lock())
}

pub fn cmd_tokens(files: Vec<String>, preview: usize) -> anyhow::Result<()> {
    let mut out = stdout_writer();
    if files.is_empty() {
        let text = read_stdin()?;
        writeln!(
            out,
            "{{\"tokens\": {}, \"chars\": {}}}",
            estimate_tokens(&text),
            text.len()
        )?;
        out.flush()?;
        return Ok(());
    }
    let mut total = 0usize;
    for f in &files {
        let text = std::fs::read_to_string(f)?;
        let t = estimate_tokens(&text);
        total += t;
        writeln!(out, "{t:>8}  {f}")?;
        if preview > 0 {
            let pv = text[..head_byte_end(&text, preview)].replace('\n', "\\n");
            writeln!(out, "  preview: {pv}")?;
        }
    }
    if files.len() > 1 {
        writeln!(out, "{total:>8}  (total)")?;
    }
    out.flush()?;
    Ok(())
}

pub fn cmd_prune(max_tokens: usize, file: Option<String>) -> anyhow::Result<()> {
    let text = match file {
        Some(f) => std::fs::read_to_string(&f)?,
        None => read_stdin()?,
    };
    // Borrow the content field when present; otherwise the raw text.
    // The old code cloned the whole input here on the common path.
    let parsed: Option<serde_json::Value> = serde_json::from_str(&text).ok();
    let raw: &str = parsed
        .as_ref()
        .and_then(|v| v.get("content"))
        .and_then(|c| c.as_str())
        .filter(|c| !c.is_empty())
        .unwrap_or(&text);
    let before = estimate_tokens(raw);
    let pruned = prune_with_total(raw, max_tokens, before);
    let after = estimate_tokens(&pruned);
    eprintln!("[rdsh] tokens {before} -> {after} (budget {max_tokens})");
    let mut out = stdout_writer();
    writeln!(out, "{pruned}")?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_counts() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("hello world, token test"), 6);
    }

    #[test]
    fn non_ascii_counts() {
        assert_eq!(estimate_tokens("a"), 1);
        assert!(estimate_tokens("hello") > 0);
        let mixed = "abcXdef";
        assert_eq!(estimate_tokens(mixed), 2);
    }

    #[test]
    fn prune_keeps_small() {
        let s = "short text";
        assert_eq!(prune_to_budget(s, 4000), s);
    }

    #[test]
    fn prune_marks_big() {
        let s: String = "x".repeat(20000);
        let p = prune_to_budget(&s, 100);
        assert!(p.contains("rdsh pruned"));
        assert!(estimate_tokens(&p) <= 200);
    }
}
