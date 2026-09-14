"""Check original beamline bytes and keep the corpus out of crates.io archives.

Run without arguments to verify the retained measurement, license and metadata
files. Pass --package to inspect Cargo's actual package file list instead.
Neither operation downloads measurement data or modifies the collection.
"""

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "crates/rexafs/tests/fixtures/xas"
FORMAT_CORPUS = ROOT / "crates/rexafs/tests/fixtures/rexafs-corpus"


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
        subprocess.run(
            [sys.executable, str(FORMAT_CORPUS / "scripts/corpus.py"), "verify", "--include-candidates"],
            cwd=ROOT,
            check=True,
        )
        snapshot = json.loads((FORMAT_CORPUS / "SNAPSHOT.json").read_text())
        for record in snapshot["files"]:
            path = (FORMAT_CORPUS / record["path"]).resolve()
            if not path.is_relative_to(FORMAT_CORPUS.resolve()):
                sys.exit(f"Corpus snapshot path escapes its directory: {record['path']}")
            payload = path.read_bytes()
            if len(payload) != record["bytes"] or hashlib.sha256(payload).hexdigest() != record["sha256"]:
                sys.exit(f"Copied corpus original bytes changed: {record['path']}")
        print(f"Verified {len(snapshot['files'])} copied data, documentation and provenance files.")
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
        or path.startswith("tests/fixtures/rexafs-corpus/")
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
