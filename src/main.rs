use clap::{Parser, Subcommand};
mod compact;
mod dsh_args;
mod guard;
mod inspect;
mod passthrough;
mod search;
mod serve;
mod slim;
mod tokens;

#[derive(Parser, Debug)]
#[command(
    name = "rdsh",
    version,
    about = "Rust fast launcher for dsh (safe: native fast-paths + passthrough)"
)]
struct Cli {
    #[arg(long = "passthrough", global = true)]
    passthrough: bool,
    #[arg(long = "dry-run", global = true)]
    dry_run: bool,
    #[arg(long = "slim", default_value_t = true, global = true)]
    slim: bool,
    #[arg(long = "no-slim", global = true)]
    no_slim: bool,
    #[arg(long = "patch", global = true)]
    patch: Vec<String>,
    #[arg(long = "profile", global = true)]
    profile: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    extra: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Boot {
        #[arg(long = "profile")]
        profile: Option<String>,
        #[arg(long = "from-default-profile")]
        from_default_profile: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(name = "dump-config")]
    DumpConfig {
        #[arg(long = "profile")]
        profile: Option<String>,
        #[arg(long = "native")]
        native: bool,
    },
    Tokens {
        files: Vec<String>,
        #[arg(long = "preview", default_value_t = 0)]
        preview: usize,
    },
    Prune {
        #[arg(long = "max-tokens", default_value_t = 4000)]
        max_tokens: usize,
        file: Option<String>,
    },
    Search {
        pattern: String,
        #[arg(long = "dir", default_value = ".")]
        dir: String,
        #[arg(long = "max", default_value_t = 100)]
        max: usize,
    },
    Compact {
        file: String,
        #[arg(long = "max-tokens", default_value_t = 8000)]
        max_tokens: usize,
    },
    Doctor,
    /// List sessions under $DSH_HOME (newest first, Node-free)
    Sessions {
        #[arg(long = "project")]
        project: Option<String>,
        #[arg(long = "limit", default_value_t = 20)]
        limit: usize,
        /// Estimate tokens via zstd decompression (falls back to stored-bytes/4)
        #[arg(long = "tokens")]
        tokens: bool,
    },
    /// List local profiles (Node-free)
    Profiles,
    /// List installed skills (Node-free)
    Skills,
    /// Show $DSH_HOME logs: latest file tail + optional grep (Node-free)
    Logs {
        #[arg(long = "tail", default_value_t = 50)]
        tail: usize,
        #[arg(long = "grep")]
        grep: Option<String>,
        #[arg(long = "file")]
        file: Option<String>,
    },
    /// Start the local dashboard (127.0.0.1 only, read-only API)
    Serve {
        #[arg(long = "port", default_value_t = 3080)]
        port: u16,
    },
    Bench {
        #[arg(long = "n", default_value_t = 5)]
        n: u32,
    },
    /// Hook helper for hooks.json: block stdin text matching --deny (exit 2)
    Guard {
        #[arg(long = "deny")]
        deny: Vec<String>,
        #[arg(long = "reason")]
        reason: Option<String>,
        #[arg(long = "json")]
        json: bool,
    },
}

/// First-arg subcommands owned by rdsh. When installed as `dsh`, anything else
/// is delegated verbatim to the original binary (so `dsh --profile tui`,
/// `dsh --version`, `dsh --help` stay byte-identical).
/// NOTE: a profile literally named like these (bare `dsh tokens`) is shadowed;
/// boot it with `dsh --profile tokens` instead.
const NATIVE_FIRST: &[&str] = &[
    "boot",
    "dump-config",
    "tokens",
    "prune",
    "search",
    "compact",
    "doctor",
    "bench",
    "serve",
    "sessions",
    "profiles",
    "skills",
    "logs",
    "guard",
];

fn invoked_as_dsh() -> bool {
    let argv0 = std::env::args().next().unwrap_or_default();
    std::path::Path::new(&argv0)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s == "dsh")
        .unwrap_or(false)
}

fn main() {
    if invoked_as_dsh() {
        let raw: Vec<String> = std::env::args().skip(1).collect();
        let first_is_native = raw
            .first()
            .map(|s| NATIVE_FIRST.contains(&s.as_str()))
            .unwrap_or(false);
        if !first_is_native {
            let slim = !passthrough::env_passthrough();
            let dry = passthrough::env_dry();
            if let Err(e) = passthrough::exec_raw(&raw, dry, slim) {
                eprintln!("[rdsh] error: {e:#}");
                std::process::exit(1);
            }
            return;
        }
        // else: fall through to the normal CLI (rdsh-native subcommand)
    }
    // NOTE: --version/-V is served by clap itself (prints "rdsh x.y.z", exit 0).
    let cli = Cli::parse();
    let slim = cli.slim && !cli.no_slim && !cli.passthrough && !passthrough::env_passthrough();
    let dry = cli.dry_run || passthrough::env_dry();
    let result: anyhow::Result<()> = match cli.command {
        Some(Commands::Tokens { files, preview }) => tokens::cmd_tokens(files, preview),
        Some(Commands::Prune { max_tokens, file }) => tokens::cmd_prune(max_tokens, file),
        Some(Commands::Search { pattern, dir, max }) => search::cmd_search(&pattern, &dir, max),
        Some(Commands::Compact { file, max_tokens }) => compact::cmd_compact(&file, max_tokens),
        Some(Commands::Doctor) => doctor(),
        Some(Commands::Sessions {
            project,
            limit,
            tokens,
        }) => inspect::cmd_sessions(project, limit, tokens),
        Some(Commands::Profiles) => inspect::cmd_profiles(),
        Some(Commands::Skills) => inspect::cmd_skills(),
        Some(Commands::Logs { tail, grep, file }) => inspect::cmd_logs(tail, grep, file),
        Some(Commands::Serve { port }) => serve::cmd_serve(port),
        Some(Commands::Bench { n }) => bench(n),
        Some(Commands::Guard { deny, reason, json }) => guard::cmd_guard(deny, reason, json),
        Some(Commands::DumpConfig { profile, native }) => {
            let p = profile.or(cli.profile).unwrap_or_else(|| "tui".to_string());
            if native {
                dump_config_native(&p, &cli.patch)
            } else {
                passthrough::exec_dump_config(&p, &cli.patch, dry, slim)
            }
        }
        Some(Commands::Boot {
            profile,
            from_default_profile,
            args,
        }) => {
            let p = profile.or(cli.profile).unwrap_or_else(|| "tui".to_string());
            passthrough::exec_boot(
                &p,
                from_default_profile.as_deref(),
                &cli.patch,
                &args,
                dry,
                slim,
            )
        }
        None => {
            let parsed = dsh_args::split_launcher_args(cli.profile, cli.extra);
            match parsed {
                dsh_args::Launcher::Help => {
                    print_help();
                    Ok(())
                }
                dsh_args::Launcher::Plugin { profile, pnpm_args } => {
                    passthrough::exec_plugin(&profile, &pnpm_args, dry, slim)
                }
                dsh_args::Launcher::Boot {
                    profile,
                    from_default,
                    patches,
                    app_args,
                } => {
                    let mut all = cli.patch;
                    all.extend(patches);
                    passthrough::exec_boot(
                        &profile,
                        from_default.as_deref(),
                        &all,
                        &app_args,
                        dry,
                        slim,
                    )
                }
                dsh_args::Launcher::Dump { profile, patches } => {
                    let mut all = cli.patch;
                    all.extend(patches);
                    passthrough::exec_dump_config(&profile, &all, dry, slim)
                }
                dsh_args::Launcher::Error(msg) => {
                    eprintln!("{msg}");
                    std::process::exit(2);
                }
            }
        }
    };
    if let Err(e) = result {
        eprintln!("[rdsh] error: {e:#}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!("rdsh: fast Rust launcher for dsh (safe shim)");
    println!();
    println!("USAGE:");
    println!("  rdsh [profile] [--profile <name>] [--patch <yml>...] [app-args...]");
    println!("  rdsh <native-subcommand> ...   (tokens|prune|search|compact|doctor|bench|serve|sessions|profiles|skills|logs|guard|dump-config|boot)");
    println!();
    println!("EXAMPLES:");
    println!("  rdsh tui                        boot tui profile (slim env ON, delegates to dsh)");
    println!("  rdsh --profile web --patch x.yml boot web with overlay");
    println!("  rdsh dump-config --profile tui  delegate exact dump to dsh");
    println!("  rdsh tokens ./AGENTS.md         estimate input tokens natively");
    println!("  rdsh search TODO --dir .        fast file search without Node");
    println!("  rdsh --passthrough tui          byte-identical delegation, no slim env");
    println!("  rdsh --dry-run tui -- --resume abc   show what would exec");
}

fn dump_config_native(profile: &str, patches: &[String]) -> anyhow::Result<()> {
    let home = std::env::var("DSH_HOME").unwrap_or_else(|_| {
        let h = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{h}/.dsh")
    });
    println!("{{\"profile\": \"{profile}\", \"dsh_home\": \"{home}\", \"patches\": {patches:?}}}");
    let root = format!("{home}/profiles/{profile}");
    match std::fs::read_dir(&root) {
        Ok(entries) => {
            println!("# layers under {root}:");
            let mut names: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            for n in names.iter().take(50) {
                println!("  - {n}");
            }
        }
        Err(_) => {
            println!("# no local profile dir at {root} (shipped template will be used by dsh)")
        }
    }
    Ok(())
}

fn doctor() -> anyhow::Result<()> {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // `let _` on purpose: piping to `grep -q` / `head` closes early (EPIPE),
    // which must not abort doctor with a panic.
    let mut say = |s: String| {
        let _ = writeln!(out, "{s}");
    };
    let orig = passthrough::find_original_dsh();
    say(format!(
        "[rdsh] original dsh: {}",
        orig.as_deref().unwrap_or("<not found in PATH>")
    ));
    let home = std::env::var("DSH_HOME").unwrap_or_else(|_| {
        let h = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{h}/.dsh")
    });
    say(format!("[rdsh] DSH_HOME: {home}"));
    match std::fs::read_dir(format!("{home}/profiles")) {
        Ok(d) => say(format!("[rdsh] profiles: {} local profile(s)", d.count())),
        Err(e) => say(format!("[rdsh] profiles: (unreadable: {e})")),
    }
    say(format!("[rdsh] slim env: {}", slim::describe()));
    if let Some(v) = original_version(orig.as_deref().unwrap_or("")) {
        say(format!("[rdsh] dsh version: {v}"));
    }
    say(format!("[rdsh] smart-dsh: {}", smart_dsh_status(&home)));
    if shadowing_original() {
        say(
            "[rdsh] note: 'dsh' currently resolves to rdsh; Smart-DSH scripts that locate"
                .to_string(),
        );
        say(
            "[rdsh] note: DSH via PATH need the original: use `dsh-orig` or set DSH_PACKAGE_DIR"
                .to_string(),
        );
    }
    for w in node_wrapper_warnings() {
        say(w);
    }
    say("[rdsh] note: dsh web GUI and `rdsh serve` both default to 3080; co-use with".to_string());
    say("[rdsh] note: `rdsh serve --port 38080` while dsh web keeps 3080".to_string());
    if orig.is_none() {
        anyhow::bail!("original 'dsh' not found; set DSH_ORIG_BIN or install @deepseek-ai/dsh");
    }
    Ok(())
}

/// Best-effort `dsh --version` readout for doctor (never fails the command).
fn original_version(orig: &str) -> Option<String> {
    if orig.is_empty() {
        return None;
    }
    let out = if orig.ends_with(".js") {
        std::process::Command::new("node")
            .arg(orig)
            .arg("--version")
            .output()
            .ok()?
    } else {
        std::process::Command::new(orig)
            .arg("--version")
            .output()
            .ok()?
    };
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Smart-DSH (https://github.com/hikarioyama/Smart-DSH) is a DSH web-profile
/// plugin bundle, not a competing binary: report which of its bundles the web
/// profile currently wires in (read-only package.json scan).
fn smart_dsh_status(home: &str) -> String {
    const KNOWN: &[&str] = &["dsh-notify-push", "dsh-esc-stop", "dsh-btw", "dsh-tasks"];
    let pkg = format!("{home}/profiles/web/package.json");
    let text = std::fs::read_to_string(&pkg).unwrap_or_default();
    if text.is_empty() {
        return "(not installed: no web profile package.json)".to_string();
    }
    let found: Vec<&str> = KNOWN.iter().copied().filter(|b| text.contains(b)).collect();
    if found.is_empty() {
        "(not installed in web profile)".to_string()
    } else {
        format!("bundles in web profile: {}", found.join(", "))
    }
}

/// Wrappers that run `node` on the `dsh` path break once `dsh` is shadowed by
/// the native binary (Node tries to parse the ELF as JS). Scan the local bin
/// dir for text files mentioning both and point at the offending wrappers.
fn node_wrapper_warnings() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return vec![];
    }
    let dir = format!("{home}/.local/bin");
    let entries = std::fs::read_dir(&dir).ok();
    let mut hits: Vec<String> = vec![];
    if let Some(entries) = entries {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap_or_default();
            if bytes.len() > 65536 || bytes.contains(&0) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            if text.contains("node") && text.contains("dsh") {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    hits.push(name.to_string());
                }
            }
            if hits.len() >= 10 {
                break;
            }
        }
    }
    hits.sort();
    let shadowed = shadowing_original();
    hits.into_iter()
        .map(|name| {
            if shadowed {
                format!("[rdsh] warn: ~/.local/bin/{name} calls `node` on a dsh path; while `dsh` is shadowed that path is a native binary — exec `dsh`/`rdsh` directly instead of via `node`")
            } else {
                format!("[rdsh] note: ~/.local/bin/{name} calls `node` on a dsh path; it will break if `dsh` is later shadowed — exec `dsh`/`rdsh` directly instead of via `node`")
            }
        })
        .collect()
}

/// True when `dsh` on PATH resolves to this binary (install.sh --as-dsh state).
fn shadowing_original() -> bool {
    if invoked_as_dsh() {
        return true;
    }
    let me = std::fs::canonicalize(std::env::current_exe().unwrap_or_default()).unwrap_or_default();
    std::env::var("PATH")
        .ok()
        .map(|p| {
            p.split(':').any(|dir| {
                let cand = format!("{dir}/dsh");
                std::fs::canonicalize(&cand)
                    .map(|c| c == me)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn bench(n: u32) -> anyhow::Result<()> {
    use std::time::Instant;
    let me = std::env::current_exe()?;
    let mut mine = vec![];
    for _ in 0..n {
        let t = Instant::now();
        let st = std::process::Command::new(&me).arg("--version").status()?;
        if !st.success() {
            anyhow::bail!("self --version failed");
        }
        mine.push(t.elapsed());
    }
    println!("[rdsh] rdsh --version x{n}: {}", summarize(&mine));
    if let Some(orig) = passthrough::find_original_dsh() {
        let mut theirs = vec![];
        for _ in 0..n {
            let t = Instant::now();
            let _ = std::process::Command::new(&orig)
                .arg("--version")
                .status()?;
            theirs.push(t.elapsed());
        }
        println!("[dsh ] dsh --version x{n}: {}", summarize(&theirs));
    } else {
        println!("[dsh ] skipped (original not found)");
    }
    Ok(())
}

fn summarize(v: &[std::time::Duration]) -> String {
    let mut s = v.to_vec();
    s.sort();
    let mid = s[s.len() / 2];
    format!("median={mid:?} min={:?} max={:?}", s[0], s[s.len() - 1])
}
