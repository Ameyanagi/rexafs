"""Check original beamline bytes and keep the corpus out of crates.io archives.

Run without arguments to verify the retained measurement, license and metadata
files. Pass --package to inspect Cargo's actual package file list instead.
Neither operation downloads measurement data or modifies the collection.
"""

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "crates/rexafs/tests/fixtures/xas"


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
    )
    if leaked:
        sys.exit("Beamline fixtures must not ship through crates.io:\n" + "\n".join(leaked))
    print("Cargo package excludes the beamline corpus and its integration test.")


if __name__ == "__main__":
    main()
