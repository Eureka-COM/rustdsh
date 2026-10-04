// rdsh guard: tiny hook command for hooks.json (Claude Code / Codex bridges).
// Scans stdin (hook JSON or raw text) for deny patterns and blocks on match.
// Exit 2 = block with reason on stderr; anything else = allow. ~1ms startup.

pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" || pattern.is_empty() {
        return true;
    }
    let parts: Vec<&str> = pattern.split("*").collect();
    let mut rest = text;
    let mut first = true;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            first = false;
            continue;
        }
        if first && i == 0 {
            if !rest.starts_with(part) {
                return false;
            }
            rest = &rest[part.len()..];
        } else if i == parts.len() - 1 && !pattern.ends_with("*") {
            if !rest.ends_with(part) {
                return false;
            }
        } else if let Some(pos) = rest.find(part) {
            rest = &rest[pos + part.len()..];
        } else {
            return false;
        }
        first = false;
    }
    true
}
fn collect_text(raw: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => {
            let mut out = String::new();
            push_strings(&v, &mut out);
            if out.is_empty() {
                raw.to_string()
            } else {
                out
            }
        }
        Err(_) => raw.to_string(),
    }
}

fn push_strings(v: &serde_json::Value, out: &mut String) {
    match v {
        serde_json::Value::String(s) => {
            out.push_str(s);
            out.push(chr_nl());
        }
        serde_json::Value::Array(a) => {
            for x in a {
                push_strings(x, out);
            }
        }
        serde_json::Value::Object(m) => {
            for x in m.values() {
                push_strings(x, out);
            }
        }
        _ => {}
    }
}

fn chr_nl() -> char {
    10 as char
}
pub fn cmd_guard(deny: Vec<String>, reason: Option<String>, json_out: bool) -> anyhow::Result<()> {
    use std::io::Read;
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    let text = collect_text(&raw);
    let hit = deny.iter().find(|p| wildcard_match(p, &text));
    match hit {
        Some(p) => {
            let msg = reason.unwrap_or_else(|| "blocked by rdsh guard".to_string());
            eprintln!("[rdsh guard] pattern hit: {}", p);
            if json_out {
                println!("{}", serde_json::json!({"decision": "block", "reason": msg}));
                Ok(())
            } else {
                eprintln!("{}", msg);
                std::process::exit(2);
            }
        }
        None => {
            if json_out {
                println!("{}", serde_json::json!({"decision": "approve"}));
            }
            Ok(())
        }
    }
}
