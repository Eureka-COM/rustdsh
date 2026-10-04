/// Slim mode: env-only trims. Additive and safe.
/// Original dsh ignores unknown DSH_* keys, so this cannot break boot.
pub fn slim_env() -> Vec<(String, String)> {
    vec![
        ("DSH_SLIM".into(), "1".into()),
        ("DSH_LAZY_PLUGINS".into(), "1".into()),
        ("DSH_DISABLE_VOICE".into(), "1".into()),
        ("DSH_DISABLE_AUTO_REVIEW".into(), "1".into()),
        ("DSH_TOKEN_BUDGET".into(), "8000".into()),
        ("DSH_TOOL_RESULT_BUDGET".into(), "4000".into()),
    ]
}

pub fn describe() -> String {
    "DSH_SLIM=1 DSH_LAZY_PLUGINS=1 DSH_DISABLE_VOICE=1 DSH_DISABLE_AUTO_REVIEW=1".to_string()
}
