//! Unified settings backed by `$DSH_HOME/rdsh.json` (schema v1).
//!
//! `serde_json` + `anyhow` + `std` のみを使います。
//! ファイル欠落・壊れ JSON は既定値にフォールバックし、不正値は clamp/切詰めで吸収します。
//! 旧 `rdsh-context.json` は読み取り専用で context 節の補完に使います（書き込みません）。

/// rdsh.json のスキーマ版。
pub const SCHEMA_VERSION: u32 = 1;

const MAX_ITEMS: usize = 50;
const GOAL_CHARS: usize = 2000;
const TEXT_CHARS: usize = 500;
const PATH_CHARS: usize = 300;
const PROFILE_CHARS: usize = 200;
const URL_CHARS: usize = 2000;

// ---- top-level ----

/// rdsh.json 全体 (schema v1)。
#[derive(Debug, Clone, PartialEq)]
pub struct RdshSettings {
    pub schema: u32,
    pub general: GeneralSection,
    pub tokens: TokensSection,
    pub search: SearchSection,
    pub compact: CompactSection,
    pub sessions: SessionsSection,
    pub logs: LogsSection,
    pub serve: ServeSection,
    pub guard: GuardSection,
    pub bench: BenchSection,
    pub setup: SetupSection,
    pub beta: BetaSection,
    pub context: ContextSection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneralSection {
    pub slim: bool,
    pub passthrough: bool,
    pub dry_run: bool,
    pub default_profile: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokensSection {
    pub default_budget: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchSection {
    pub dir: String,
    pub max: usize,
    pub web_limit: usize,
    pub searxng_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompactSection {
    pub max_tokens: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionsSection {
    pub limit: usize,
    pub with_tokens: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogsSection {
    pub tail: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServeSection {
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuardSection {
    pub deny: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchSection {
    pub n: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetupSection {
    /// 0 = ランダムポート。
    pub web_port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BetaSection {
    pub context_engine: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextSection {
    pub token_budget: usize,
    pub enable_retriever: bool,
    pub enable_packer: bool,
    pub enable_verifier: bool,
    pub goal: String,
    pub decisions: Vec<String>,
    pub constraints: Vec<String>,
    pub working_files: Vec<String>,
    pub open_tasks: Vec<String>,
    pub max_code_hits: usize,
    pub max_sessions: usize,
    pub include_git_diff: bool,
}

impl Default for GeneralSection {
    fn default() -> Self {
        Self { slim: true, passthrough: false, dry_run: false, default_profile: String::new() }
    }
}

impl Default for TokensSection {
    fn default() -> Self {
        Self { default_budget: 4000 }
    }
}

impl Default for SearchSection {
    fn default() -> Self {
        Self { dir: ".".to_string(), max: 100, web_limit: 10, searxng_url: String::new() }
    }
}

impl Default for CompactSection {
    fn default() -> Self {
        Self { max_tokens: 8000 }
    }
}

impl Default for SessionsSection {
    fn default() -> Self {
        Self { limit: 20, with_tokens: false }
    }
}

impl Default for LogsSection {
    fn default() -> Self {
        Self { tail: 50 }
    }
}

impl Default for ServeSection {
    fn default() -> Self {
        Self { port: 3080 }
    }
}

impl Default for GuardSection {
    fn default() -> Self {
        Self { deny: vec![], reason: String::new() }
    }
}

impl Default for BenchSection {
    fn default() -> Self {
        Self { n: 5 }
    }
}

impl Default for SetupSection {
    fn default() -> Self {
        Self { web_port: 0 }
    }
}

impl Default for BetaSection {
    fn default() -> Self {
        Self { context_engine: true }
    }
}

impl Default for ContextSection {
    fn default() -> Self {
        Self {
            token_budget: 4000,
            enable_retriever: true,
            enable_packer: true,
            enable_verifier: true,
            goal: String::new(),
            decisions: vec![],
            constraints: vec![],
            working_files: vec![],
            open_tasks: vec![],
            max_code_hits: 20,
            max_sessions: 10,
            include_git_diff: true,
        }
    }
}

impl Default for RdshSettings {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            general: GeneralSection::default(),
            tokens: TokensSection::default(),
            search: SearchSection::default(),
            compact: CompactSection::default(),
            sessions: SessionsSection::default(),
            logs: LogsSection::default(),
            serve: ServeSection::default(),
            guard: GuardSection::default(),
            bench: BenchSection::default(),
            setup: SetupSection::default(),
            beta: BetaSection::default(),
            context: ContextSection::default(),
        }
    }
}

// ---- public API (Lead の main 配線用: シグネチャ厳守) ----

/// `$DSH_HOME/rdsh.json` のパス。
pub fn settings_path() -> String {
    format!("{}/rdsh.json", crate::inspect::dsh_home())
}

/// 設定を読み込む。欠落・壊れ JSON は既定値、不正値は clamp/切詰めで吸収する。
/// context 節が空なら旧 rdsh-context.json で補完する（読み取り専用）。
pub fn load() -> RdshSettings {
    let raw = std::fs::read_to_string(settings_path()).unwrap_or_default();
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
    let has_context = parsed.get("context").is_some_and(|c| !c.is_null());
    let mut cfg = if parsed.is_null() {
        RdshSettings::default()
    } else {
        RdshSettings::from_value(&parsed)
    };
    complement_from_legacy(&mut cfg, has_context);
    cfg
}

impl RdshSettings {
    /// `mkdir -p` した上で mode 600 相当で保存する。
    pub fn save(&self) -> anyhow::Result<()> {
        let path = settings_path();
        if let Some(parent) = std::path::Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut clean = self.clone();
        clean.sanitize();
        let text = serde_json::to_string_pretty(&clean.to_value())?;
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true).mode(0o600);
            let mut f = opts.open(&path)?;
            f.write_all(text.as_bytes())?;
            f.write_all(b"\n")?;
            drop(f);
            let mut perm = std::fs::metadata(&path)?.permissions();
            perm.set_mode(0o600);
            std::fs::set_permissions(&path, perm)?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&path, format!("{text}\n"))?;
        }
        Ok(())
    }

    fn from_value(v: &serde_json::Value) -> Self {
        let sec = |key: &str| -> serde_json::Value {
            v.get(key).cloned().unwrap_or(serde_json::Value::Null)
        };
        let g = sec("general");
        let t = sec("tokens");
        let s = sec("search");
        let c = sec("compact");
        let ss = sec("sessions");
        let l = sec("logs");
        let sv = sec("serve");
        let gd = sec("guard");
        let b = sec("bench");
        let su = sec("setup");
        let be = sec("beta");
        let cx = sec("context");
        let mut out = Self {
            schema: SCHEMA_VERSION,
            general: GeneralSection {
                slim: flag(&g, "slim", true),
                passthrough: flag(&g, "passthrough", false),
                dry_run: flag(&g, "dry_run", false),
                default_profile: text(&g, "default_profile", PROFILE_CHARS),
            },
            tokens: TokensSection {
                default_budget: clamp_u64(num(&t, "default_budget"), 4000, 500, 200000) as usize,
            },
            search: SearchSection {
                dir: {
                    let d = text(&s, "dir", PATH_CHARS);
                    if d.trim().is_empty() { ".".to_string() } else { d }
                },
                max: clamp_u64(num(&s, "max"), 100, 1, 100) as usize,
                web_limit: clamp_u64(num(&s, "web_limit"), 10, 1, 100) as usize,
                searxng_url: text(&s, "searxng_url", URL_CHARS),
            },
            compact: CompactSection {
                max_tokens: clamp_u64(num(&c, "max_tokens"), 8000, 500, 200000) as usize,
            },
            sessions: SessionsSection {
                limit: clamp_u64(num(&ss, "limit"), 20, 1, 100) as usize,
                with_tokens: flag(&ss, "with_tokens", false),
            },
            logs: LogsSection {
                tail: clamp_u64(num(&l, "tail"), 50, 1, 500) as usize,
            },
            serve: ServeSection {
                port: clamp_u64(num(&sv, "port"), 3080, 1, 65535) as u16,
            },
            guard: GuardSection {
                deny: list(&gd, "deny", TEXT_CHARS),
                reason: text(&gd, "reason", TEXT_CHARS),
            },
            bench: BenchSection {
                n: clamp_u64(num(&b, "n"), 5, 1, 20) as u32,
            },
            setup: SetupSection {
                web_port: match num(&su, "web_port") {
                    None => 0,
                    Some(0) => 0,
                    Some(n) => n.clamp(1, 65535) as u16,
                },
            },
            beta: BetaSection {
                context_engine: flag(&be, "context_engine", true),
            },
            context: ContextSection {
                token_budget: clamp_u64(num(&cx, "token_budget"), 4000, 500, 200000) as usize,
                enable_retriever: flag(&cx, "enable_retriever", true),
                enable_packer: flag(&cx, "enable_packer", true),
                enable_verifier: flag(&cx, "enable_verifier", true),
                goal: text(&cx, "goal", GOAL_CHARS),
                decisions: list(&cx, "decisions", TEXT_CHARS),
                constraints: list(&cx, "constraints", TEXT_CHARS),
                working_files: {
                    let w = list(&cx, "working_files", PATH_CHARS);
                    if w.is_empty() { list(&cx, "files", PATH_CHARS) } else { w }
                },
                open_tasks: list(&cx, "open_tasks", TEXT_CHARS),
                max_code_hits: clamp_u64(num(&cx, "max_code_hits"), 20, 1, 100) as usize,
                max_sessions: clamp_u64(num(&cx, "max_sessions"), 10, 1, 100) as usize,
                include_git_diff: flag(&cx, "include_git_diff", true),
            },
        };
        out.sanitize();
        out
    }

    /// 不正値の吸収（to_value 経由の正規化）。
    fn sanitize(&mut self) {
        let v = self.to_value();
        *self = Self::from_value_raw(&v);
    }

    /// sanitize の再帰を避ける素朴な正規化。
    fn from_value_raw(v: &serde_json::Value) -> Self {
        let mut out = Self::default();
        out.schema = SCHEMA_VERSION;
        let g = v.get("general").cloned().unwrap_or(serde_json::Value::Null);
        out.general.slim = flag(&g, "slim", true);
        out.general.passthrough = flag(&g, "passthrough", false);
        out.general.dry_run = flag(&g, "dry_run", false);
        out.general.default_profile = text(&g, "default_profile", PROFILE_CHARS);
        let t = v.get("tokens").cloned().unwrap_or(serde_json::Value::Null);
        out.tokens.default_budget = clamp_u64(num(&t, "default_budget"), 4000, 500, 200000) as usize;
        let s = v.get("search").cloned().unwrap_or(serde_json::Value::Null);
        out.search.dir = {
            let d = text(&s, "dir", PATH_CHARS);
            if d.trim().is_empty() { ".".to_string() } else { d }
        };
        out.search.max = clamp_u64(num(&s, "max"), 100, 1, 100) as usize;
        out.search.web_limit = clamp_u64(num(&s, "web_limit"), 10, 1, 100) as usize;
        out.search.searxng_url = text(&s, "searxng_url", URL_CHARS);
        let c = v.get("compact").cloned().unwrap_or(serde_json::Value::Null);
        out.compact.max_tokens = clamp_u64(num(&c, "max_tokens"), 8000, 500, 200000) as usize;
        let ss = v.get("sessions").cloned().unwrap_or(serde_json::Value::Null);
        out.sessions.limit = clamp_u64(num(&ss, "limit"), 20, 1, 100) as usize;
        out.sessions.with_tokens = flag(&ss, "with_tokens", false);
        let l = v.get("logs").cloned().unwrap_or(serde_json::Value::Null);
        out.logs.tail = clamp_u64(num(&l, "tail"), 50, 1, 500) as usize;
        let sv = v.get("serve").cloned().unwrap_or(serde_json::Value::Null);
        out.serve.port = clamp_u64(num(&sv, "port"), 3080, 1, 65535) as u16;
        let gd = v.get("guard").cloned().unwrap_or(serde_json::Value::Null);
        out.guard.deny = list(&gd, "deny", TEXT_CHARS);
        out.guard.reason = text(&gd, "reason", TEXT_CHARS);
        let bn = v.get("bench").cloned().unwrap_or(serde_json::Value::Null);
        out.bench.n = clamp_u64(num(&bn, "n"), 5, 1, 20) as u32;
        let su = v.get("setup").cloned().unwrap_or(serde_json::Value::Null);
        out.setup.web_port = match num(&su, "web_port") {
            None => 0,
            Some(0) => 0,
            Some(n) => n.clamp(1, 65535) as u16,
        };
        let be = v.get("beta").cloned().unwrap_or(serde_json::Value::Null);
        out.beta.context_engine = flag(&be, "context_engine", true);
        let cx = v.get("context").cloned().unwrap_or(serde_json::Value::Null);
        out.context.token_budget = clamp_u64(num(&cx, "token_budget"), 4000, 500, 200000) as usize;
        out.context.enable_retriever = flag(&cx, "enable_retriever", true);
        out.context.enable_packer = flag(&cx, "enable_packer", true);
        out.context.enable_verifier = flag(&cx, "enable_verifier", true);
        out.context.goal = text(&cx, "goal", GOAL_CHARS);
        out.context.decisions = list(&cx, "decisions", TEXT_CHARS);
        out.context.constraints = list(&cx, "constraints", TEXT_CHARS);
        out.context.working_files = {
            let w = list(&cx, "working_files", PATH_CHARS);
            if w.is_empty() { list(&cx, "files", PATH_CHARS) } else { w }
        };
        out.context.open_tasks = list(&cx, "open_tasks", TEXT_CHARS);
        out.context.max_code_hits = clamp_u64(num(&cx, "max_code_hits"), 20, 1, 100) as usize;
        out.context.max_sessions = clamp_u64(num(&cx, "max_sessions"), 10, 1, 100) as usize;
        out.context.include_git_diff = flag(&cx, "include_git_diff", true);
        out
    }

    fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": self.schema,
            "general": {
                "slim": self.general.slim,
                "passthrough": self.general.passthrough,
                "dry_run": self.general.dry_run,
                "default_profile": self.general.default_profile,
            },
            "tokens": { "default_budget": self.tokens.default_budget },
            "search": {
                "dir": self.search.dir,
                "max": self.search.max,
                "web_limit": self.search.web_limit,
                "searxng_url": self.search.searxng_url,
            },
            "compact": { "max_tokens": self.compact.max_tokens },
            "sessions": { "limit": self.sessions.limit, "with_tokens": self.sessions.with_tokens },
            "logs": { "tail": self.logs.tail },
            "serve": { "port": self.serve.port },
            "guard": { "deny": self.guard.deny, "reason": self.guard.reason },
            "bench": { "n": self.bench.n },
            "setup": { "web_port": self.setup.web_port },
            "beta": { "context_engine": self.beta.context_engine },
            "context": {
                "token_budget": self.context.token_budget,
                "enable_retriever": self.context.enable_retriever,
                "enable_packer": self.context.enable_packer,
                "enable_verifier": self.context.enable_verifier,
                "goal": self.context.goal,
                "decisions": self.context.decisions,
                "constraints": self.context.constraints,
                "working_files": self.context.working_files,
                "open_tasks": self.context.open_tasks,
                "max_code_hits": self.context.max_code_hits,
                "max_sessions": self.context.max_sessions,
                "include_git_diff": self.context.include_git_diff,
            },
        })
    }
}

// ---- legacy (旧 rdsh-context.json: 読み取り専用) ----

fn legacy_context_path() -> String {
    format!("{}/rdsh-context.json", crate::inspect::dsh_home())
}

/// context 節が空（未設定または既定値のまま）なら旧ファイルで項目ごとに補完する。
/// 旧ファイルへの書き込みはしない。
fn complement_from_legacy(cfg: &mut RdshSettings, has_context: bool) {
    if has_context && cfg.context != ContextSection::default() {
        return;
    }
    let raw = std::fs::read_to_string(legacy_context_path()).unwrap_or_default();
    if raw.trim().is_empty() {
        return;
    }
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return,
    };
    if v.is_null() {
        return;
    }
    let c = &mut cfg.context;
    let budget = clamp_u64(num(&v, "token_budget"), 4000, 500, 200000) as usize;
    if c.token_budget == ContextSection::default().token_budget && budget != c.token_budget {
        c.token_budget = budget;
    }
    if c.enable_retriever && !flag(&v, "enable_retriever", true) {
        c.enable_retriever = false;
    }
    if c.enable_packer && !flag(&v, "enable_packer", true) {
        c.enable_packer = false;
    }
    if c.enable_verifier && !flag(&v, "enable_verifier", true) {
        c.enable_verifier = false;
    }
    if c.goal.is_empty() {
        c.goal = text(&v, "goal", GOAL_CHARS);
    }
    if c.decisions.is_empty() {
        c.decisions = list(&v, "decisions", TEXT_CHARS);
    }
    if c.constraints.is_empty() {
        c.constraints = list(&v, "constraints", TEXT_CHARS);
    }
    if c.working_files.is_empty() {
        let w = list(&v, "working_files", PATH_CHARS);
        c.working_files = if w.is_empty() { list(&v, "files", PATH_CHARS) } else { w };
    }
    if c.open_tasks.is_empty() {
        c.open_tasks = list(&v, "open_tasks", TEXT_CHARS);
    }
    if c.max_code_hits == ContextSection::default().max_code_hits {
        if let Some(n) = num(&v, "max_code_hits") {
            c.max_code_hits = n.clamp(1, 100) as usize;
        }
    }
    if c.max_sessions == ContextSection::default().max_sessions {
        if let Some(n) = num(&v, "max_sessions") {
            c.max_sessions = n.clamp(1, 100) as usize;
        }
    }
}

// ---- small JSON helpers ----

fn num(v: &serde_json::Value, key: &str) -> Option<u64> {
    v.get(key).and_then(|x| x.as_u64())
}

fn flag(v: &serde_json::Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(default)
}

fn text(v: &serde_json::Value, key: &str, max_chars: usize) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| truncate(s, max_chars))
        .unwrap_or_default()
}

fn list(v: &serde_json::Value, key: &str, max_chars: usize) -> Vec<String> {
    match v.get(key).and_then(|x| x.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|x| x.as_str())
            .map(|s| truncate(s, max_chars))
            .filter(|s: &String| !s.trim().is_empty())
            .take(MAX_ITEMS)
            .collect(),
        None => vec![],
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

fn clamp_u64(n: Option<u64>, default: u64, lo: u64, hi: u64) -> u64 {
    n.unwrap_or(default).clamp(lo, hi)
}

#[cfg(test)]
mod rdsh_config_tests {
    use super::*;

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        static M: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        M.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// DSH_HOME を一時 dir に向けて f を実行する（復元・後片付けつき）。
    fn with_home(tag: &str, f: impl FnOnce(&str)) {
        let _guard = lock();
        let dir = std::env::temp_dir().join(format!("rdsh-settings-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let prev = std::env::var("DSH_HOME").ok();
        std::env::set_var("DSH_HOME", &dir);
        f(&dir.to_string_lossy());
        match prev {
            Some(p) => std::env::set_var("DSH_HOME", p),
            None => std::env::remove_var("DSH_HOME"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn defaults_match_spec() {
        let d = RdshSettings::default();
        assert_eq!(d.schema, 1);
        assert!(d.general.slim && !d.general.passthrough && !d.general.dry_run);
        assert_eq!(d.general.default_profile, "");
        assert_eq!(d.tokens.default_budget, 4000);
        assert_eq!(d.search.dir, ".");
        assert_eq!((d.search.max, d.search.web_limit), (100, 10));
        assert_eq!(d.search.searxng_url, "");
        assert_eq!(d.compact.max_tokens, 8000);
        assert_eq!((d.sessions.limit, d.sessions.with_tokens), (20, false));
        assert_eq!(d.logs.tail, 50);
        assert_eq!(d.serve.port, 3080);
        assert!(d.guard.deny.is_empty() && d.guard.reason.is_empty());
        assert_eq!(d.bench.n, 5);
        assert_eq!(d.setup.web_port, 0);
        assert!(d.beta.context_engine);
        assert_eq!(d.context.token_budget, 4000);
        assert!(d.context.enable_retriever && d.context.enable_packer && d.context.enable_verifier);
        assert!(d.context.goal.is_empty());
        assert!(d.context.decisions.is_empty() && d.context.constraints.is_empty());
        assert!(d.context.working_files.is_empty() && d.context.open_tasks.is_empty());
        assert_eq!((d.context.max_code_hits, d.context.max_sessions), (20, 10));
        assert!(d.context.include_git_diff);
    }

    #[test]
    fn missing_file_returns_default() {
        with_home("missing", |_| {
            assert!(!std::path::Path::new(&settings_path()).exists());
            assert_eq!(load(), RdshSettings::default());
        });
    }

    #[test]
    fn broken_json_returns_default() {
        with_home("broken", |_| {
            let p = settings_path();
            std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
            std::fs::write(&p, "{oops,,,").unwrap();
            assert_eq!(load(), RdshSettings::default());
        });
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        with_home("clamp", |_| {
            let p = settings_path();
            std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
            std::fs::write(
                &p,
                serde_json::json!({
                    "tokens": { "default_budget": 9999999 },
                    "compact": { "max_tokens": 1 },
                    "context": { "token_budget": 10 },
                    "search": { "max": 500, "web_limit": 0 },
                    "sessions": { "limit": 0 },
                    "logs": { "tail": 9999 },
                    "bench": { "n": 99 },
                    "serve": { "port": 0 },
                    "setup": { "web_port": 70000 },
                })
                .to_string(),
            )
            .unwrap();
            let c = load();
            assert_eq!(c.tokens.default_budget, 200000);
            assert_eq!(c.compact.max_tokens, 500);
            assert_eq!(c.context.token_budget, 500);
            assert_eq!(c.search.max, 100);
            assert_eq!(c.search.web_limit, 1);
            assert_eq!(c.sessions.limit, 1);
            assert_eq!(c.logs.tail, 500);
            assert_eq!(c.bench.n, 20);
            assert_eq!(c.serve.port, 1);
            assert_eq!(c.setup.web_port, 65535);
        });
    }

    #[test]
    fn setup_web_port_zero_means_random() {
        with_home("webport", |_| {
            let p = settings_path();
            std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
            std::fs::write(&p, serde_json::json!({ "setup": { "web_port": 0 } }).to_string())
                .unwrap();
            assert_eq!(load().setup.web_port, 0);
        });
    }

    #[test]
    fn long_strings_and_lists_are_truncated() {
        with_home("trunc", |_| {
            let p = settings_path();
            std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
            let big_goal: String = "あ".repeat(2500);
            let big_path: String = "x".repeat(400);
            let many: Vec<String> = (0..60).map(|i| format!("dec-{i}")).collect();
            std::fs::write(
                &p,
                serde_json::json!({
                    "context": {
                        "goal": big_goal,
                        "decisions": many,
                        "working_files": [big_path],
                    },
                })
                .to_string(),
            )
            .unwrap();
            let c = load();
            assert_eq!(c.context.goal.chars().count(), 2000);
            assert_eq!(c.context.decisions.len(), 50);
            assert_eq!(c.context.working_files[0].chars().count(), 300);
        });
    }

    #[test]
    fn save_roundtrip_and_mode_600() {
        with_home("roundtrip", |dir| {
            let mut cfg = RdshSettings::default();
            cfg.general.default_profile = "tui".to_string();
            cfg.context.goal = "dsh互換性を維持する".to_string();
            cfg.context.working_files = vec!["src/search.rs".to_string()];
            cfg.save().unwrap();
            let p = settings_path();
            assert!(std::path::Path::new(&p).exists());
            assert!(p.starts_with(dir));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600, "save は mode 600 相当のはず");
            }
            assert_eq!(load(), cfg);
        });
    }

    #[test]
    fn legacy_file_completes_empty_context_without_writing() {
        with_home("legacy", |_| {
            let legacy = legacy_context_path();
            std::fs::create_dir_all(std::path::Path::new(&legacy).parent().unwrap()).unwrap();
            std::fs::write(
                &legacy,
                serde_json::json!({
                    "token_budget": 6000,
                    "enable_verifier": false,
                    "goal": "旧ファイルのgoal",
                    "files": ["src/tokens.rs"],
                    "open_tasks": ["packing評価"],
                })
                .to_string(),
            )
            .unwrap();
            // rdsh.json は作らない: 補完は読み取り専用。
            let c = load();
            assert_eq!(c.context.token_budget, 6000);
            assert!(!c.context.enable_verifier);
            assert!(c.context.enable_retriever && c.context.enable_packer);
            assert_eq!(c.context.goal, "旧ファイルのgoal");
            assert_eq!(c.context.working_files, vec!["src/tokens.rs".to_string()]);
            assert_eq!(c.context.open_tasks, vec!["packing評価".to_string()]);
            assert!(!std::path::Path::new(&settings_path()).exists());
        });
    }
}
