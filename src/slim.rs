// Slim mode: env-only trims. Additive and safe.
// Namespace rule (matches dsh convention: DSH_* is owned by dsh itself,
// so rdsh-private keys live under RDSH_*). The original dsh ignores keys
// it does not know, so these hints cannot break boot.
pub fn slim_env() -> Vec<(String, String)> {
    vec![
        ("RDSH_SLIM".into(), "1".into()),
        ("RDSH_LAZY_PLUGINS".into(), "1".into()),
        ("RDSH_DISABLE_VOICE".into(), "1".into()),
        ("RDSH_DISABLE_AUTO_REVIEW".into(), "1".into()),
        ("RDSH_TOKEN_BUDGET".into(), "8000".into()),
        ("RDSH_TOOL_RESULT_BUDGET".into(), "4000".into()),
    ]
}

pub fn describe() -> String {
    "RDSH_SLIM=1 RDSH_LAZY_PLUGINS=1 RDSH_DISABLE_VOICE=1 RDSH_DISABLE_AUTO_REVIEW=1".to_string()
}
