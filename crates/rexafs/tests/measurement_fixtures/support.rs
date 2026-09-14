//! Shared locations for fixture-backed tests; no downloads or regeneration.
use std::path::PathBuf;

pub(super) fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub(super) fn xas_root() -> PathBuf {
    fixture_root().join("xas")
}

pub(super) fn format_corpus_root() -> PathBuf {
    fixture_root().join("rexafs-corpus")
}

pub(super) fn session_root(format: &str) -> PathBuf {
    fixture_root().join("sessions").join(format)
}
