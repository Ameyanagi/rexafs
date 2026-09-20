"""Require every artifact for one registry to match the qualified build manifest."""
import argparse
import hashlib
from pathlib import Path
import re

from python_wheels import check_wheel, inventory


def read_manifest(path):
    """Read a flat SHA256SUMS file into a basename-to-hex-digest dictionary.

    Entries must use lowercase 64-character digests and unique names without
    path separators. Malformed or repeated entries raise ValueError. No files
    are modified or fetched.
    """
    entries = {}
    for line in path.read_text().splitlines():
        digest, name = line.split("  ", 1)
        if (not re.fullmatch(r"[0-9a-f]{64}", digest) or not name
                or "/" in name or "\\" in name or name in entries):
            raise ValueError("Invalid or duplicate artifact manifest entry")
        entries[name] = digest
    return entries


def required_names(channel, version, entries, python_abi="per-interpreter"):
    """Select all manifest entries required for one coordinated registry release.

    Cargo and npm require one source crate or tarball. PyPI requires the source
    archive and every wheel in the qualified manifest. The ABI3 source profile
    additionally requires the version's exact platform inventory: three wheels
    from 0.2.12, four for earlier ABI3 releases. Historical per-interpreter tags
    retain their manifest-defined inventory. Cargo alpha/beta/rc suffixes become Python's
    a/b/rc spelling. Unsupported versions/channels or missing assets raise
    ValueError. The input manifest is left unchanged.
    """
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
        if python_abi == "abi3-py310":
            inventory(wheels, version)
        elif python_abi != "per-interpreter" or any("-abi3-" in name for name in wheels):
            raise ValueError("Wheel ABI differs from the immutable source tag")
        names = wheels | {f"rexafs-{python_version}.tar.gz"}
    else:
        raise ValueError("Unsupported registry")
    if not names <= entries.keys():
        raise ValueError("The manifest is missing required registry artifacts")
    return names


def verify(channel, artifacts, manifest, version, python_abi="per-interpreter"):
    """Return the verified file count for one registry's downloaded artifacts.

    Recursively hash files below artifacts, excluding manifest itself, and
    require exact equality with required_names. Unexpected files and duplicate
    basenames fail, even when required packages are also present. This read-only
    check permits each registry to be resumed without downloading unrelated
    desktop assets; it does not authenticate the original GitHub run.
    """
    entries = read_manifest(manifest)
    names = required_names(channel, version, entries, python_abi)
    actual = {}
    for path in artifacts.rglob("*"):
        if not path.is_file() or path.resolve() == manifest.resolve():
            continue
        if path.name in actual:
            raise ValueError("Duplicate registry artifact name: " + path.name)
        with path.open("rb") as stream:
            actual[path.name] = hashlib.file_digest(stream, "sha256").hexdigest()
        if channel == "pypi" and python_abi == "abi3-py310" and path.suffix == ".whl":
            check_wheel(path, version)
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
    parser.add_argument("--python-abi", choices=["per-interpreter", "abi3-py310"], default="per-interpreter")
    args = parser.parse_args()
    count = verify(args.channel, args.artifacts, args.manifest, args.version, args.python_abi)
    print(f"Verified all {count} {args.channel} artifacts against the qualified GitHub build")
