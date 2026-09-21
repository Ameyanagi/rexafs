#!/usr/bin/env python3
"""Extract the bundled synthetic copper inputs for numerical reproduction.

Uses only the Python standard library. The source project is read without
modification. The destination must be new; all embedded bytes and SHA-256
checksums are verified before any output is written. No spectra are regenerated
or processed. This helper accepts the bundled example's 53 spectra and two
metadata files, not arbitrary portable projects.
"""

import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath


def extract(project, output):
    document = json.loads(project.read_text())
    expected = {f"data/references/{s}.xdi" for s in ("CuO", "Cu2O", "Cu")}
    expected |= {f"data/series/frame_{i:02}.xdi" for i in range(1, 51)}
    expected |= {"manifest.json", "fractions.csv"}
    prepared = {}
    for entry in document["header"]["files"]:
        relative = entry["path"]
        if relative not in expected or relative in prepared:
            raise ValueError(f"Unexpected or duplicate example member: {relative}")
        content = gzip.decompress(base64.b64decode(document["embedded"][entry["sha256"]], validate=True))
        if len(content) != entry["bytes"] or hashlib.sha256(content).hexdigest() != entry["sha256"]:
            raise ValueError(f"Embedded content verification failed: {relative}")
        prepared[relative] = content
    if set(prepared) != expected:
        raise ValueError("The bundled example is incomplete")
    output.mkdir(parents=True, exist_ok=False)
    for relative, content in prepared.items():
        destination = output.joinpath(*PurePosixPath(relative).parts)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(content)
    print(f"Verified and extracted 53 spectra and 2 metadata files to {output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    extract(args.project, args.output)
