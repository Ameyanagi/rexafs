"""Check coordinated source and lockfile versions, optionally against a release tag.

The Cargo workspace owns the version. Python derives it through maturin; npm
duplicates it in its manifest and lockfile. This checks those contracts without
building packages, changing files or querying registries. Website release.json
describes published downloads and intentionally advances only after publication.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

import tomllib

MEMBERS = {
    "crates/rexafs": "rexafs",
    "crates/rexafs-gui": "rexafs-gui",
    "crates/rexafs-wasm": "rexafs-wasm",
    "py-rexafs": "py-rexafs",
}
NUMBER = r"(?:0|[1-9][0-9]*)"
VERSION = re.compile(rf"{NUMBER}\.{NUMBER}\.{NUMBER}(?:-(?:alpha|beta|rc)\.{NUMBER})?")


def require_equal(actual: object, expected: object, label: str) -> None:
    """Raise ValueError with the field name and expected value on a mismatch."""
    if actual != expected:
        raise ValueError(f"{label}: expected {expected!r}, found {actual!r}")


def validate(root: Path, tag: str | None = None) -> str:
    """Return the coordinated version after checking source and lockfile metadata.

    Accept stable versions or alpha/beta/rc prereleases with numeric suffixes.
    All four Rust members must inherit the workspace version, and the Python
    project must derive its version from that workspace's binding crate. Cargo's
    local package entries and both npm lockfile version fields must agree.
    If supplied, tag must equal ``v`` plus the version. Missing or inconsistent
    metadata raises ValueError, KeyError or the corresponding file/parse error.
    This does not validate dependency resolution or published artifact contents;
    locked builds and publication checks remain separate release gates.
    """
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))[
        "workspace"
    ]
    version = workspace["package"]["version"]
    if not isinstance(version, str) or not VERSION.fullmatch(version):
        raise ValueError(f"Unsupported coordinated release version: {version!r}")
    dependency = workspace["dependencies"]["rexafs"]
    require_equal(
        dependency["version"],
        version,
        "Cargo.toml workspace.dependencies.rexafs.version",
    )
    require_equal(
        dependency["path"],
        "crates/rexafs",
        "Cargo.toml workspace.dependencies.rexafs.path",
    )

    for directory, name in MEMBERS.items():
        package = tomllib.loads(
            (root / directory / "Cargo.toml").read_text(encoding="utf-8")
        )["package"]
        require_equal(package["name"], name, f"{directory}/Cargo.toml package.name")
        require_equal(
            package["version"],
            {"workspace": True},
            f"{directory}/Cargo.toml package.version",
        )

    locked = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))["package"]
    for name in MEMBERS.values():
        entries = [package for package in locked if package["name"] == name]
        require_equal(len(entries), 1, f"Cargo.lock {name} package count")
        require_equal(entries[0]["version"], version, f"Cargo.lock {name} version")
        require_equal(
            entries[0].get("source"), None, f"Cargo.lock {name} source (must be local)"
        )

    npm = json.loads((root / "js-rexafs/package.json").read_text(encoding="utf-8"))
    npm_lock = json.loads(
        (root / "js-rexafs/package-lock.json").read_text(encoding="utf-8")
    )
    for label, package in [
        ("js-rexafs/package.json", npm),
        ("js-rexafs/package-lock.json", npm_lock),
        ("js-rexafs/package-lock.json packages['']", npm_lock["packages"][""]),
    ]:
        require_equal(package["name"], "rexafs", f"{label} name")
        require_equal(package["version"], version, f"{label} version")

    python = tomllib.loads((root / "pyproject.toml").read_text(encoding="utf-8"))
    require_equal(python["project"]["name"], "rexafs", "pyproject.toml project.name")
    if "version" in python["project"] or "version" not in python["project"].get(
        "dynamic", []
    ):
        raise ValueError(
            "pyproject.toml must derive project.version dynamically from Cargo"
        )
    require_equal(
        python["build-system"]["build-backend"],
        "maturin",
        "pyproject.toml build-backend",
    )
    require_equal(
        python["tool"]["maturin"]["manifest-path"],
        "py-rexafs/Cargo.toml",
        "pyproject.toml tool.maturin.manifest-path",
    )
    if tag is not None:
        require_equal(tag, f"v{version}", "release tag")
    return version


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "tag", nargs="?", help="Expected v-prefixed coordinated source tag"
    )
    args = parser.parse_args()
    try:
        print(validate(Path(__file__).resolve().parents[1], args.tag))
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f"Release version check failed: {error}\n")


if __name__ == "__main__":
    main()
