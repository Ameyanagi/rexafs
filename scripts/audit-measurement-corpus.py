"""Audit a separately licensed fixture corpus using the installed, unreleased reader.

Run with Python 3.12+ after installing the source-checkout Python wheel:
    python scripts/audit-measurement-corpus.py --corpus crates/rexafs/tests/fixtures/xas --collection rexafs-corpus --output report.json

Checks every manifest payload's size and SHA-256 before parsing. Records recovered
scans, numeric datasets, warnings and conversion errors for each detected signal.
It does not download, alter, normalize or redistribute source measurements.
Successful parsing is not proof of complete recovery or scientific correctness;
independent numerical regressions remain necessary for each new format.
"""

import argparse
import hashlib
import importlib.metadata
import importlib.util
import json
import platform
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

from rexafs.io import read_measurement


def retained_path(root, paths, historical_path):
    """Resolve collection aliases without changing historical report paths."""
    path = (root / (paths[historical_path] if paths is not None else historical_path)).resolve()
    if not path.is_relative_to(root):
        raise ValueError(f"Path outside corpus: {historical_path}")
    return path


def inspect(root, record, paths=None):
    path = retained_path(root, paths, record["path"])
    payload = path.read_bytes()
    if len(payload) != record["bytes"] or hashlib.sha256(payload).hexdigest() != record["sha256"]:
        raise ValueError(f"Source integrity mismatch: {record['path']}")
    del payload
    result = {key: record.get(key) for key in (
        "path", "sha256", "bytes", "facility", "beamline", "format", "data_level",
        "license", "license_basis", "test_eligibility", "source_url",
    )}
    try:
        measurement = read_measurement(path)
        document = measurement.document
    except (ValueError, OSError, RuntimeError) as error:
        return {**result, "status": "rejected", "error": str(error)}
    scans = []
    for index, scan in enumerate(document["scans"]):
        signals = []
        for signal in scan["signals"]:
            try:
                energy, mu = measurement.arrays(scan=index, mapping=signal["mapping"])
                signals.append({"name": signal["name"], "points": len(energy), "status": "converted"})
            except (ValueError, IndexError, RuntimeError) as error:
                signals.append({"name": signal["name"], "status": "rejected", "error": str(error)})
        scans.append({
            "id": scan["id"], "columns": len(scan["columns"]),
            "points": len(scan["columns"][0]["values"]) if scan["columns"] else 0,
            "signals": signals, "warnings": scan["warnings"],
        })
    partial = any("cannot enumerate" in warning for warning in document["warnings"])
    converted = sum(signal["status"] == "converted" for scan in scans for signal in scan["signals"])
    status = "partial" if partial else "signal_choices" if converted else "mapping_or_reduction_required"
    return {
        **result, "status": status, "detected_format": document["format"],
        "scans": scans, "datasets": len(document["datasets"]),
        "converted_signals": converted, "warnings": document["warnings"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument(
        "--collection", choices=["rexafs-corpus"],
        help="Read the expanded historical manifest through its canonical path map",
    )
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = args.corpus.resolve()
    paths = None
    if args.collection:
        paths = json.loads((root / "collections" / args.collection / "paths.json").read_text())
    manifest_bytes = retained_path(root, paths, "manifest.json").read_bytes()
    manifest = json.loads(manifest_bytes)
    records = [inspect(root, record, paths) for record in manifest["samples"]]
    report = {
        "scope": "Content parsing and detected-signal conversion; not complete-format or scientific qualification.",
        "audited_at": datetime.now(timezone.utc).isoformat(),
        "reader_build": {
            "distribution_version": importlib.metadata.version("rexafs"),
            "python_version": platform.python_version(),
            "extension_sha256": hashlib.sha256(
                Path(importlib.util.find_spec("rexafs._core").origin).read_bytes()
            ).hexdigest(),
            "audit_script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        },
        "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
        "files": len(records), "statuses": dict(Counter(record["status"] for record in records)),
        "records": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key != "records"}, indent=2))


if __name__ == "__main__":
    main()
