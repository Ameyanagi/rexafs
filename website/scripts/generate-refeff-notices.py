"""Retain exact license texts for the pinned ReFEFF WASI Cargo dependency graph.

This maintenance command requires Python 3.12+, Git and Cargo. It reads an
immutable Git archive, verifies registry archives against that archive's lockfile,
and does not compile or modify ReFEFF. Website builds use its checked-in outputs.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import re
import subprocess
import tarfile
import tempfile
import urllib.request
from pathlib import Path
from typing import Any, TypedDict

import tomllib

COMMIT = "91b23364309c3daca871b748ae45bf2426c60b8e"
REPOSITORY = "https://github.com/Ameyanagi/refeff"
TARGET = "wasm32-wasip1"
WEBSITE = Path(__file__).resolve().parents[1]
TREE_COMMAND = [
    "cargo",
    "tree",
    "--locked",
    "--target",
    TARGET,
    "-p",
    "refeff-cli",
    "--edges",
    "normal,build",
    "--prefix",
    "none",
    "--format",
    "{p}",
    "--no-dedupe",
]
METADATA_COMMAND = [
    "cargo",
    "metadata",
    "--locked",
    "--format-version",
    "1",
    "--filter-platform",
    TARGET,
    "--manifest-path",
    "crates/refeff-cli/Cargo.toml",
]


class Supplement(TypedDict):
    packages: list[str]
    url: str
    sha256: str


# These crates omit their primary license text from the registry archive. Each
# URL uses the VCS commit recorded inside that exact, lockfile-verified archive.
SUPPLEMENTS: list[Supplement] = [
    {
        "packages": ["equator@0.2.2"],
        "url": "https://raw.githubusercontent.com/sarah-ek/equator/9d4107bdd6a75598aed4a60fe46200c9e9f4e653/LICENSE",
        "sha256": "97d3424e347e8fa236806725ce9837bb453d4dfc9ce1d4733fd6023a8038000a",
    },
    {
        "packages": ["equator-macro@0.2.1"],
        "url": "https://raw.githubusercontent.com/sarah-ek/equator/c9061f3da9a7cc53b0d388f26ab6bb95422823c3/LICENSE",
        "sha256": "97d3424e347e8fa236806725ce9837bb453d4dfc9ce1d4733fd6023a8038000a",
    },
    {
        "packages": ["faer@0.24.0", "faer-traits@0.24.0"],
        "url": "https://codeberg.org/sarah-quinones/faer/raw/commit/8dfcceee8eb3d81231366acc9d3f157ac5e7057a/LICENSE",
        "sha256": "1f6b7e2a30afc2f5f253cb3535e041f91de1e1f9815b38714dfd94e53e61d972",
    },
    {
        "packages": ["nano-gemm@0.2.2"],
        "url": "https://raw.githubusercontent.com/sarah-ek/nano-gemm/806b012a6f5ee0615042bf64675180fb97ff2ccc/LICENSE",
        "sha256": "310b7323adcc27fc94c45d1155b6c89c17986576d790f766a04c855662428a7c",
    },
    {
        "packages": [
            "nano-gemm-c32@0.2.1",
            "nano-gemm-c64@0.2.1",
            "nano-gemm-codegen@0.2.1",
            "nano-gemm-core@0.2.1",
            "nano-gemm-f32@0.2.1",
            "nano-gemm-f64@0.2.1",
        ],
        "url": "https://raw.githubusercontent.com/sarah-ek/nano-gemm/d5f1cd77f0af31fb90aa0d128a76e2366c5dabc7/LICENSE",
        "sha256": "310b7323adcc27fc94c45d1155b6c89c17986576d790f766a04c855662428a7c",
    },
    {
        "packages": ["pulp-wasm-simd-flag@0.1.0"],
        "url": "https://raw.githubusercontent.com/sarah-quinones/pulp/28ba9b9a75da773d9426f18019fed63eff0e591f/LICENSE",
        "sha256": "d64f878c89bd5f1e5ada7e4aad57690b122727cf5229b7b87ea1980c188da1d9",
    },
]


def sha256(data: bytes | bytearray) -> str:
    return hashlib.sha256(data).hexdigest()


def command(arguments: list[str], root: Path) -> bytes:
    return subprocess.check_output(arguments, cwd=root)


def notice_name(path: str) -> bool:
    return Path(path).name.upper().startswith(("LICENSE", "COPYING", "NOTICE"))


def registry_files(package: dict, lock: dict) -> tuple[dict[str, bytes], str]:
    """Read authenticated crate bytes, rather than trusting an editable cache."""
    directory = Path(package["manifest_path"]).parent
    archive_path = (
        directory.parents[2]
        / "cache"
        / directory.parent.name
        / f"{package['name']}-{package['version']}.crate"
    )
    data = archive_path.read_bytes()
    expected = lock["checksum"]
    if sha256(data) != expected:
        raise ValueError(f"Registry archive checksum mismatch: {archive_path}")
    prefix = f"{package['name']}-{package['version']}/"
    files = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive.getmembers():
            if not member.isfile():
                continue
            if not member.name.startswith(prefix) or ".." in Path(member.name).parts:
                raise ValueError(f"Unexpected crate archive member: {member.name}")
            relative = member.name.removeprefix(prefix)
            if relative in files:
                raise ValueError(f"Duplicate crate archive member: {member.name}")
            stream = archive.extractfile(member)
            if stream is None:
                raise ValueError(f"Cannot read crate archive member: {member.name}")
            files[relative] = stream.read()
    # The resolved declaration must agree with the checksum-verified manifest.
    declaration = tomllib.loads(files["Cargo.toml"].decode())["package"]
    for field in ("name", "version", "license", "repository"):
        if declaration.get(field) != package.get(field):
            raise ValueError(
                f"Cargo metadata disagrees with archive: {package['name']} {field}"
            )
    if declaration.get("license-file") != package.get("license_file"):
        raise ValueError(
            f"Cargo metadata disagrees with archive license-file: {package['name']}"
        )
    return files, expected


def generate(repository: Path, output: Path, check: bool) -> None:
    texts: dict[str, bytes] = {}
    uses: dict[str, list[str]] = {}
    packages = []
    with tempfile.TemporaryDirectory(prefix="rexafs-refeff-notices-") as temporary:
        source = Path(temporary).resolve()
        archive_bytes = command(["git", "archive", COMMIT], repository)
        with tarfile.open(fileobj=io.BytesIO(archive_bytes)) as archive:
            # The source archive is generated by Git at the fixed commit; do not
            # extract archive-provided links or paths outside this temporary tree.
            for member in archive.getmembers():
                if (
                    member.issym()
                    or member.islnk()
                    or ".." in Path(member.name).parts
                    or member.name.startswith("/")
                ):
                    raise ValueError(f"Unsafe source archive member: {member.name}")
            archive.extractall(source, filter="data")
        metadata = json.loads(command(METADATA_COMMAND, source))
        selected = set()
        for line in command(TREE_COMMAND, source).decode().splitlines():
            match = re.fullmatch(r"(\S+) v(\S+)(?: \([^\n]+\))?", line)
            if match is None:
                raise ValueError(f"Unexpected Cargo tree output: {line}")
            selected.add(match.groups())
        lock_bytes = (source / "Cargo.lock").read_bytes()
        lock_packages = tomllib.loads(lock_bytes.decode())["package"]
        matches = [
            p for p in metadata["packages"] if (p["name"], p["version"]) in selected
        ]
        if len(matches) != len(selected):
            raise ValueError(
                "Cargo tree package selection is absent or ambiguous in metadata"
            )
        source_hashes = {
            path: sha256((source / path).read_bytes())
            for path in ("Cargo.toml", "Cargo.lock", "wasm/build.mjs")
        }
        for package in sorted(matches, key=lambda p: (p["name"], p["version"])):
            name = f"{package['name']}@{package['version']}"
            entry: dict[str, Any] = {
                key: package.get(key)
                for key in ("name", "version", "license", "repository", "source")
            }
            entry["licenseFile"] = package.get("license_file")
            directory = Path(package["manifest_path"]).parent
            if package["source"]:
                candidates = [
                    p
                    for p in lock_packages
                    if all(
                        p.get(key) == package.get(key)
                        for key in ("name", "version", "source")
                    )
                ]
                if len(candidates) != 1:
                    raise ValueError(f"Missing or ambiguous lockfile entry: {name}")
                files, digest = registry_files(package, candidates[0])
                entry["archiveSha256"] = digest
                entry["archiveUrl"] = (
                    f"https://static.crates.io/crates/{package['name']}/{package['name']}-{package['version']}.crate"
                )
                entry["manifestPath"] = "Cargo.toml"
                source_kind = "registry-archive"
            else:
                files = {
                    p.relative_to(directory).as_posix(): p.read_bytes()
                    for p in directory.rglob("*")
                    if p.is_file()
                }
                entry["sourceCommit"] = COMMIT
                entry["manifestPath"] = (
                    (directory / "Cargo.toml").relative_to(source).as_posix()
                )
                source_kind = "refeff-source"
            entry["manifestSha256"] = sha256(files["Cargo.toml"])
            entry["vcs"] = (
                json.loads(files[".cargo_vcs_info.json"])
                if ".cargo_vcs_info.json" in files
                else None
            )
            retained = []

            def retain(
                package_name: str,
                records: list,
                path: str,
                data: bytes,
                kind: str,
                url: str | None = None,
            ) -> None:
                data.decode("utf-8")  # Keep readable UTF-8 text and original bytes.
                digest = sha256(data)
                texts[digest] = data
                uses.setdefault(digest, []).append(f"{package_name}: {path} ({kind})")
                record = {
                    "path": path,
                    "sourceKind": kind,
                    "sha256": digest,
                    "bytes": len(data),
                }
                if url is not None:
                    record["sourceUrl"] = url
                records.append(record)

            for path, data in sorted(files.items()):
                if notice_name(path) or path == entry["licenseFile"]:
                    url = (
                        f"{REPOSITORY}/blob/{COMMIT}/{directory.relative_to(source).as_posix()}/{path}"
                        if source_kind == "refeff-source"
                        else None
                    )
                    retain(name, retained, path, data, source_kind, url)
            if entry["licenseFile"] and entry["licenseFile"] not in files:
                raise ValueError(f"Declared license-file is missing: {name}")
            for supplement in SUPPLEMENTS:
                if name not in supplement["packages"]:
                    continue
                vcs = entry["vcs"]
                if not isinstance(vcs, dict) or not isinstance(vcs.get("git"), dict):
                    raise TypeError(
                        f"Missing crate VCS record for supplemental license: {name}"
                    )
                vcs_commit = vcs["git"].get("sha1")
                if not isinstance(vcs_commit, str) or not re.fullmatch(
                    r"[a-f0-9]{40}", vcs_commit
                ):
                    raise ValueError(
                        f"Invalid crate VCS commit for supplemental license: {name}"
                    )
                if f"/{vcs_commit}/" not in supplement["url"]:
                    raise ValueError(
                        f"Supplement does not match the crate VCS commit: {name}"
                    )
                with urllib.request.urlopen(supplement["url"], timeout=60) as response:
                    data = response.read()
                if sha256(data) != supplement["sha256"]:
                    raise ValueError(f"Supplemental license checksum mismatch: {name}")
                retain(
                    name, retained, "LICENSE", data, "crate-vcs-root", supplement["url"]
                )
            if not retained:
                raise ValueError(
                    f"No license text retained for selected package: {name}"
                )
            entry["files"] = retained
            packages.append(entry)

    bundle = bytearray(
        (
            "ReFEFF 0.4.0 WASI: Rust dependency license texts\n"
            f"Source: {REPOSITORY}/tree/{COMMIT}\n"
            f"Target: {TARGET}; root package: refeff-cli 0.3.0.\n\n"
            "This bundle retains exact license, COPYING and NOTICE file bytes for the\n"
            "selected Cargo normal/build graph, including procedural macros and build\n"
            "dependencies. It is a conservative attribution inventory, not a claim\n"
            "that every listed package contributes code to the final WebAssembly.\n"
            "Cargo dev-only dependencies are excluded. Rust standard-library/compiler\n"
            "runtime and WASI libc components are outside this Cargo-package inventory.\n"
            "ReFEFF/FEFF10 and browser WASI shim notices are also supplied separately.\n\n"
            "Identical file contents are retained once, with every package/path listed.\n"
            "The JSON inventory records source versions, provenance and SHA-256 hashes;\n"
            "its byte offsets identify each unchanged text in this UTF-8 bundle.\n"
            "Separators and this introduction were added by rexafs.\n"
        ).encode()
    )
    offsets = {}
    for digest, data in sorted(texts.items()):
        header = "\n" + "=" * 78 + f"\nSHA-256: {digest}\nApplies to:\n"
        header += "".join(f"  {name}\n" for name in sorted(uses[digest]))
        header += f"BEGIN EXACT FILE BYTES ({len(data)} bytes)\n"
        bundle.extend(header.encode())
        offsets[digest] = len(bundle)
        bundle.extend(data)
        bundle.extend(b"\nEND EXACT FILE BYTES\n")
    for package in packages:
        for record in package["files"]:
            record["bundleByteOffset"] = offsets[record["sha256"]]
    inventory = {
        "schemaVersion": 1,
        "generator": "website/scripts/generate-refeff-notices.py",
        "refeffVersion": "0.4.0",
        "sourceRepository": REPOSITORY,
        "sourceCommit": COMMIT,
        "sourceFilesSha256": source_hashes,
        "target": TARGET,
        "rootPackage": "refeff-cli@0.3.0",
        "selectionCommand": TREE_COMMAND,
        "metadataCommand": METADATA_COMMAND,
        "scope": "Selected Cargo normal/build graph; conservative with respect to final linked bytes. Dev-only dependencies and Rust toolchain/WASI libc components are excluded.",
        "missingLicenseTexts": [],
        "supplementedPackages": sorted(
            name for item in SUPPLEMENTS for name in item["packages"]
        ),
        "packageCount": len(packages),
        "uniqueTextCount": len(texts),
        "bundle": {
            "path": "RUST-NOTICES.txt",
            "sha256": sha256(bundle),
            "bytes": len(bundle),
        },
        "packages": packages,
    }
    results = {
        "RUST-NOTICES.txt": bytes(bundle),
        "rust-dependencies.json": (
            json.dumps(inventory, indent=2, ensure_ascii=False) + "\n"
        ).encode(),
    }
    if not check:
        output.mkdir(parents=True, exist_ok=True)
    for name, data in results.items():
        path = output / name
        if check:
            if path.read_bytes() != data:
                raise ValueError(f"Generated ReFEFF notices are stale: {path}")
        else:
            path.write_bytes(data)
        print(f"{sha256(data)}  {name} ({len(data)} bytes)")
    print(
        f"Retained {len(texts)} unique texts for {len(packages)} packages; no missing package license texts."
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--refeff-repository",
        required=True,
        type=Path,
        help="Local ReFEFF Git repository containing the pinned commit; read only.",
    )
    parser.add_argument("--output", type=Path, default=WEBSITE / "vendor/refeff-0.4.0")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify exact generated bytes without replacing files.",
    )
    arguments = parser.parse_args()
    generate(
        arguments.refeff_repository.resolve(),
        arguments.output.resolve(),
        arguments.check,
    )
