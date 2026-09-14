"""Locations of the one shared, separately licensed measurement collection."""
from pathlib import Path

FIXTURES = Path(__file__).resolve().parents[3] / "crates/rexafs/tests/fixtures/xas"
SESSIONS = FIXTURES.parent / "sessions"
LARIX = SESSIONS / "larix"
