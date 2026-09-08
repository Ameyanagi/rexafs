"""Require every artifact for one registry to match the qualified build manifest."""
import argparse
import hashlib
from pathlib import Path
import re


def read_manifest(path):
    entries = {}
    for line in path.read_text().splitlines():
        digest, name = line.split("  ", 1)
        if (not re.fullmatch(r"[0-9a-f]{64}", digest) or not name
                or "/" in name or "\\" in name or name in entries):
            raise ValueError("Invalid or duplicate artifact manifest entry")
        entries[name] = digest
    return entries


def required_names(channel, version, entries):
    match = re.fullmatch(r"(\d+\.\d+\.\d+)(?:-(alpha|beta|rc)\.(\d+))?", version)
    if not match:
        raise ValueError("Expected a coordinated release version")
    if channel == "crates-io":
        names = {f"rexafs-{version}.crate"}
    elif channel == "npm":
        names = {f"rexafs-{version}.tgz"}
    elif channel == "pypi":
        python_version = match[1]
        if match[2]:
            python_version += {"alpha": "a", "beta": "b", "rc": "rc"}[match[2]] + match[3]
        wheels = {name for name in entries if name.endswith(".whl")}
        if not wheels or any(not name.startswith(f"rexafs-{python_version}-") for name in wheels):
            raise ValueError("The manifest must contain wheels for the intended Python version")
        names = wheels | {f"rexafs-{python_version}.tar.gz"}
    else:
        raise ValueError("Unsupported registry")
    if not names <= entries.keys():
        raise ValueError("The manifest is missing required registry artifacts")
    return names


def verify(channel, artifacts, manifest, version):
    entries = read_manifest(manifest)
    names = required_names(channel, version, entries)
    actual = {}
    for path in artifacts.rglob("*"):
        if not path.is_file() or path.resolve() == manifest.resolve():
            continue
        if path.name in actual:
            raise ValueError("Duplicate registry artifact name: " + path.name)
        with path.open("rb") as stream:
            actual[path.name] = hashlib.file_digest(stream, "sha256").hexdigest()
    expected = {name: entries[name] for name in names}
    if actual != expected:
        raise ValueError("Registry artifacts are missing, unexpected, or differ from the qualified build")
    return len(actual)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("channel", choices=["crates-io", "npm", "pypi"])
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("version")
    args = parser.parse_args()
    count = verify(args.channel, args.artifacts, args.manifest, args.version)
    print(f"Verified all {count} {args.channel} artifacts against the qualified GitHub build")
