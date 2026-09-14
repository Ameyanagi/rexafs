//! Shared locations for fixture-backed tests; no downloads or regeneration.
use std::path::PathBuf;

pub(super) fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub(super) fn xas_root() -> PathBuf {
    fixture_root().join("xas")
}

pub(super) fn format_corpus_path(historical_path: &str) -> PathBuf {
    // Preserve the original manifest paths while sharing identical fixture bytes.
    static PATHS: std::sync::OnceLock<std::collections::BTreeMap<String, String>> =
        std::sync::OnceLock::new();
    let paths = PATHS.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../fixtures/xas/collections/rexafs-corpus/paths.json"
        ))
        .unwrap()
    });
    xas_root().join(paths.get(historical_path).expect("retained corpus path"))
}

pub(super) fn session_root(format: &str) -> PathBuf {
    fixture_root().join("sessions").join(format)
}
