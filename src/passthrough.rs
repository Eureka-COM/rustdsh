use std::os::unix::process::CommandExt;

/// Env overrides for drop-in `dsh` mode (see install.sh --as-dsh).
/// RDSH_PASSTHROUGH=1 disables slim env; RDSH_DRY_RUN=1 only prints the exec.
pub fn env_passthrough() -> bool {
    std::env::var("RDSH_PASSTHROUGH").as_deref() == Ok("1")
}

pub fn env_dry() -> bool {
    std::env::var("RDSH_DRY_RUN").as_deref() == Ok("1")
}

fn origin_file() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let p = format!("{home}/.config/rdsh/origin");
    let s = std::fs::read_to_string(p).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Locate the original Node-based dsh (never ourselves).
pub fn find_original_dsh() -> Option<String> {
    // Canonical name first, legacy DSH_ORIG_BIN kept as fallback.
    for key in ["RDSH_ORIG_BIN", "DSH_ORIG_BIN"] {
        if let Ok(p) = std::env::var(key) {
            if !p.is_empty() {
                return Some(p);
            }
        }
    }
    if let Some(p) = origin_file() {
        if std::fs::metadata(&p).is_ok() {
            return Some(p);
        }
    }
    let me = std::env::current_exe().ok();
    // Sibling backups created by install.sh --as-dsh (same dir as our binary).
    if let Some(exe) = &me {
        if let Some(dir) = exe.parent() {
            for name in ["dsh-orig", "dsh.orig", "dsh.real"] {
                let cand = dir.join(name);
                if std::fs::metadata(&cand).is_ok() {
                    return Some(cand.to_string_lossy().into_owned());
                }
            }
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let cand = format!("{dir}/dsh");
            if std::fs::metadata(&cand).is_ok() {
                let mut skip = false;
                if let (Some(m), Ok(c)) = (&me, std::fs::canonicalize(&cand)) {
                    if let Ok(m) = std::fs::canonicalize(m) {
                        skip = m == c;
                    }
                }
                if !skip {
                    return Some(cand);
                }
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        for cand in [
            format!("{home}/.local/bin/dsh.orig"),
            format!("{home}/.local/bin/dsh-orig"),
            format!("{home}/.local/opt/node-v24.16.0-linux-x64/lib/node_modules/@deepseek-ai/dsh/lib/bin.js"),
        ] {
            if std::fs::metadata(&cand).is_ok() {
                return Some(cand);
            }
        }
    }
    None
}

fn base_cmd(orig: &str) -> std::process::Command {
    if orig.ends_with(".js") {
        let mut c = std::process::Command::new("node");
        c.arg(orig);
        c
    } else {
        std::process::Command::new(orig)
    }
}

fn apply_slim(cmd: &mut std::process::Command, slim: bool) {
    if slim {
        for (k, v) in crate::slim::slim_env() {
            cmd.env(k, v);
        }
    }
}

fn exec_or_spawn(mut cmd: std::process::Command, dry: bool) -> anyhow::Result<()> {
    if dry {
        println!("[rdsh dry-run] would exec: {cmd:?}");
        return Ok(());
    }
    let err = cmd.exec();
    Err(anyhow::anyhow!("exec failed: {err}"))
}

pub fn exec_boot(
    profile: &str,
    from_default: Option<&str>,
    patches: &[String],
    app_args: &[String],
    dry: bool,
    slim: bool,
) -> anyhow::Result<()> {
    let orig = find_original_dsh()
        .ok_or_else(|| anyhow::anyhow!("original dsh not found in PATH (set DSH_ORIG_BIN)"))?;
    let mut cmd = base_cmd(&orig);
    cmd.arg("--profile").arg(profile);
    if let Some(f) = from_default {
        cmd.arg("--from-default-profile").arg(f);
    }
    for p in patches {
        cmd.arg("--patch").arg(p);
    }
    cmd.args(app_args);
    apply_slim(&mut cmd, slim);
    if slim {
        eprintln!(
            "[rdsh] boot '{profile}' via {orig} (slim ON: {})",
            crate::slim::describe()
        );
    }
    exec_or_spawn(cmd, dry)
}

pub fn exec_dump_config(
    profile: &str,
    patches: &[String],
    dry: bool,
    slim: bool,
) -> anyhow::Result<()> {
    let orig = find_original_dsh()
        .ok_or_else(|| anyhow::anyhow!("original dsh not found in PATH (set DSH_ORIG_BIN)"))?;
    let mut cmd = base_cmd(&orig);
    cmd.arg("--profile").arg(profile);
    for p in patches {
        cmd.arg("--patch").arg(p);
    }
    cmd.arg("--dump-config");
    apply_slim(&mut cmd, slim);
    exec_or_spawn(cmd, dry)
}

/// Raw verbatim delegation (used when invoked as `dsh`): no arg rewriting.
pub fn exec_raw(args: &[String], dry: bool, slim: bool) -> anyhow::Result<()> {
    let orig = find_original_dsh().ok_or_else(|| {
        anyhow::anyhow!("original dsh not found (set DSH_ORIG_BIN or reinstall with install.sh)")
    })?;
    let mut cmd = base_cmd(&orig);
    cmd.args(args);
    apply_slim(&mut cmd, slim);
    exec_or_spawn(cmd, dry)
}

pub fn exec_plugin(
    profile: &str,
    pnpm_args: &[String],
    dry: bool,
    slim: bool,
) -> anyhow::Result<()> {
    let orig = find_original_dsh()
        .ok_or_else(|| anyhow::anyhow!("original dsh not found in PATH (set DSH_ORIG_BIN)"))?;
    let mut cmd = base_cmd(&orig);
    cmd.arg("plugin").arg("--profile").arg(profile);
    cmd.args(pnpm_args);
    apply_slim(&mut cmd, slim);
    exec_or_spawn(cmd, dry)
}
