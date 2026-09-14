"""Check original beamline bytes and keep the corpus out of crates.io archives.

Run without arguments to verify the retained measurement, license and metadata
files. Pass --package to inspect Cargo's actual package file list instead.
Neither operation downloads measurement data or modifies the collection.
"""

import argparse
import hashlib
import json
import runpy
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "crates/rexafs/tests/fixtures/xas"
FORMAT_CORPUS = CORPUS / "collections/rexafs-corpus"


def retained_path(paths, historical_path):
    """Resolve an archived path to its single retained file, within the corpus."""
    path = (CORPUS / paths[historical_path]).resolve()
    if not path.is_relative_to(CORPUS.resolve()):
        sys.exit(f"Corpus path escapes its directory: {historical_path}")
    return path


def verify_expanded_collection():
    """Verify every historical file against the unchanged original snapshot."""
    paths = json.loads((FORMAT_CORPUS / "paths.json").read_text())
    snapshot = json.loads((FORMAT_CORPUS / "SNAPSHOT.json").read_text())
    expected = {record["path"] for record in snapshot["files"]} | {"SNAPSHOT.json"}
    if set(paths) != expected:
        sys.exit("Expanded corpus path map must retain every original snapshot entry")
    for record in snapshot["files"]:
        payload = retained_path(paths, record["path"]).read_bytes()
        if len(payload) != record["bytes"] or hashlib.sha256(payload).hexdigest() != record["sha256"]:
            sys.exit(f"Copied corpus original bytes changed: {record['path']}")
    # Original sidecar text stays attached to each logical measurement even when
    # two historical collections now resolve to the same physical file.
    manifest = json.loads(retained_path(paths, "manifest.json").read_text())
    # Retain the original container checks (including ZIP/XML and gzip checks)
    # without invoking the historical script's download or report-writing modes.
    original_checks = runpy.run_path(
        str(retained_path(paths, "scripts/corpus.py")), run_name="expanded_corpus"
    )
    for record in manifest["samples"]:
        sidecar = retained_path(paths, record["path"] + ".license")
        if sidecar.read_text() != record["sidecar_text"]:
            sys.exit(f"Expanded corpus attribution changed: {record['path']}")
        original_checks["inspect_payload"](record, retained_path(paths, record["path"]).read_bytes())
    print(f"Verified {len(snapshot['files'])} original corpus files through canonical paths.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", action="store_true", help="Check Cargo's package file list")
    args = parser.parse_args()
    if not args.package:
        subprocess.run(
            [sys.executable, str(CORPUS / "scripts/corpus.py"), "verify", "--include-candidates"],
            cwd=ROOT,
            check=True,
        )
        verify_expanded_collection()
        xtunes = ROOT / "crates/rexafs/tests/fixtures/sessions/xtunes"
        records = json.loads((xtunes / "manifest.json").read_text())["files"]
        for record in records:
            payload = (xtunes / record["path"]).read_bytes()
            if len(payload) != record["bytes"] or hashlib.sha256(payload).hexdigest() != record["sha256"]:
                sys.exit(f"XTUNES original bytes changed: {record['path']}")
        print(f"Verified {len(records)} XTUNES measurement and attribution files.")
        subprocess.run(
            [sys.executable, str(ROOT / "crates/rexafs/tests/fixtures/sessions/larix/verify.py")],
            cwd=ROOT,
            check=True,
        )
        return
    result = subprocess.run(
        ["cargo", "package", "--locked", "--allow-dirty", "--list", "-p", "rexafs"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    )
    paths = {line.strip().replace("\\", "/") for line in result.stdout.splitlines()}
    if not {"Cargo.toml", "src/lib.rs"} <= paths:
        sys.exit("Cargo did not return the expected rexafs package file list")
    leaked = sorted(
        path for path in paths
        if (path.startswith("tests/beamline_") and path.endswith(".rs") and path.count("/") == 1)
        or path.startswith("tests/fixtures/xas/")
        or path.startswith("tests/fixtures/sessions/")
        or path == "tests/measurement_fixtures.rs"
        or path.startswith("tests/measurement_fixtures/")
    )
    if leaked:
        sys.exit("Beamline fixtures must not ship through crates.io:\n" + "\n".join(leaked))
    required_contracts = {
        "tests/measurement_reader.rs",
        "tests/measurement_reader/contracts.rs",
        "tests/measurement_reader/larix.rs",
        "tests/measurement_reader/xtunes.rs",
    }
    if missing := required_contracts - paths:
        sys.exit("Self-contained reader contracts must remain in the crate archive:\n" + "\n".join(sorted(missing)))
    print("Cargo package excludes attributed fixtures and their test targets, and retains self-contained reader contracts.")


if __name__ == "__main__":
    main()
