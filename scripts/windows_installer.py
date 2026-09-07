"""Build an unsigned, per-user Windows installer from a qualified desktop bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any
from zipfile import ZipFile

TARGET = "x86_64-pc-windows-msvc"
REPOSITORY = "Ameyanagi/rexafs"
VERSION = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?")


def digest(path: Path) -> str:
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def bundle_identity(bundle: Path) -> dict[str, Any]:
    metadata = json.loads((bundle / "build.json").read_text(encoding="utf-8"))
    version = metadata.get("version", "")
    if not isinstance(version, str) or not VERSION.fullmatch(version):
        raise ValueError("Invalid desktop version")
    if metadata.get("target") != TARGET or metadata.get("dirty") is not False:
        raise ValueError("Installer requires a clean Windows MSVC desktop build")
    if not re.fullmatch(r"[0-9a-f]{40}", metadata.get("commit", "")):
        raise ValueError("Missing source commit")
    channel = metadata.get("channel")
    tag = metadata.get("release_tag", "")
    if channel == "stable":
        if tag != f"v{version}":
            raise ValueError("Stable desktop tag does not match its version")
    elif channel == "nightly":
        if not re.fullmatch(r"nightly-\d{8}-\d+", tag) or not metadata.get("built_at"):
            raise ValueError("Nightly desktop requires an immutable tag and timestamp")
    else:
        raise ValueError("Unknown desktop channel")
    for name in (
        "rexafs.exe",
        "resources/rexafs.ico",
        "resources/examples/cu_150k.xmu",
        "resources/examples/PROVENANCE.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "dependencies.json",
    ):
        if not (bundle / name).is_file():
            raise ValueError(f"Missing installer payload: {name}")
    if not (bundle / "licenses").is_dir():
        raise ValueError("Missing third-party license notices")
    if any(p.is_symlink() for p in bundle.rglob("*")):
        raise ValueError("Installer payload must not contain symlinks")
    return metadata


def output_name(metadata: dict[str, Any]) -> str:
    label = (
        metadata["release_tag"]
        if metadata["channel"] == "nightly"
        else metadata["version"]
    )
    return f"rexafs-{label}-{TARGET}-setup"


def extract_release(
    archive: Path, checksum: Path, destination: Path, tag: str, commit: str
) -> Path:
    """Check identity and checksum before any executable from the ZIP is run."""
    entries = [
        line.split("  ", 1) for line in checksum.read_text().splitlines() if line
    ]
    if entries != [[digest(archive), archive.name]]:
        raise ValueError("Desktop ZIP does not match its release checksum")
    stem = archive.stem
    seen: set[str] = set()
    with ZipFile(archive) as source:
        for entry in source.infolist():
            path = PurePosixPath(entry.filename)
            if (
                not path.parts
                or path.parts[0] != stem
                or path.is_absolute()
                or ".." in path.parts
                or "\\" in entry.filename
                or ":" in entry.filename
                or any(part.endswith((" ", ".")) for part in path.parts)
                or (entry.external_attr >> 16) & 0o170000 == 0o120000
                or entry.filename.rstrip("/").casefold() in seen
            ):
                raise ValueError(f"Unsafe or ambiguous archive path: {entry.filename}")
            seen.add(entry.filename.rstrip("/").casefold())
        source.extractall(destination)
    bundle = destination / stem
    metadata = bundle_identity(bundle)
    if metadata["release_tag"] != tag or metadata["commit"] != commit:
        raise ValueError("Desktop ZIP is not from the requested release tag commit")
    return bundle


def download_release(tag: str, destination: Path) -> Path:
    if not tag.startswith("v") or not VERSION.fullmatch(tag[1:]):
        raise ValueError("Use a version tag such as v0.1.3")
    destination.mkdir(parents=True, exist_ok=False)
    name = f"rexafs-{tag[1:]}-{TARGET}.zip"
    subprocess.run(
        [
            "gh",
            "release",
            "download",
            tag,
            "--repo",
            REPOSITORY,
            "--dir",
            str(destination),
            "--pattern",
            name,
            "--pattern",
            name + ".sha256",
        ],
        check=True,
    )
    commit = subprocess.check_output(
        [
            "gh",
            "api",
            f"repos/{REPOSITORY}/commits/{tag}",
            "--jq",
            ".sha",
        ],
        text=True,
    ).strip()
    bundle = extract_release(
        destination / name,
        destination / (name + ".sha256"),
        destination / "extracted",
        tag,
        commit,
    )
    return bundle


def runtime_directory() -> Path:
    vswhere = (
        Path(os.environ["ProgramFiles(x86)"])
        / "Microsoft Visual Studio/Installer/vswhere.exe"
    )
    installation = subprocess.check_output(
        [
            str(vswhere),
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ],
        text=True,
    ).strip()
    if not installation:
        raise ValueError("Visual Studio C++ redistributable directory not found")
    candidates = list(
        (Path(installation) / "VC/Redist/MSVC").glob("*/x64/Microsoft.VC*.CRT")
    )
    if not candidates:
        raise ValueError("No x64 Microsoft CRT redistributable directory")
    return max(
        candidates, key=lambda p: tuple(int(n) for n in p.parents[1].name.split("."))
    )


def signed_runtime(path: Path) -> dict[str, str]:
    # Use a process-local variable so paths are never interpolated into PowerShell code.
    command = """
    $ErrorActionPreference = 'Stop'
    $p = $env:REXAFS_RUNTIME_TO_VERIFY
    $s = Get-AuthenticodeSignature -LiteralPath $p
    if ($s.Status -ne 'Valid' -or $s.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
        throw 'Runtime must have a valid Microsoft signature'
    }
    @{version=(Get-Item -LiteralPath $p).VersionInfo.FileVersion;
      signer=$s.SignerCertificate.Subject} | ConvertTo-Json -Compress
    """
    result = json.loads(
        subprocess.check_output(
            [
                shutil.which("pwsh") or "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                command,
            ],
            text=True,
            env={**os.environ, "REXAFS_RUNTIME_TO_VERIFY": str(path)},
        )
    )
    return {**result, "sha256": digest(path)}


def build(
    bundle: Path,
    output: Path,
    runtime: Path | None = None,
    compiler: Path | None = None,
) -> Path:
    if os.name != "nt":
        raise ValueError("Compile Windows installers on Windows")
    bundle = bundle.resolve()
    metadata = bundle_identity(bundle)
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    name = output_name(metadata)
    installer = output / (name + ".exe")
    if installer.exists():
        raise ValueError("Installer output already exists")
    compiler = (
        compiler or Path(os.environ["ProgramFiles(x86)"]) / "Inno Setup 6/ISCC.exe"
    )
    runtime = runtime or runtime_directory()
    if not (runtime / "vcruntime140.dll").is_file():
        raise ValueError("Microsoft x64 vcruntime140.dll is required")
    runtime_files = {p.name: signed_runtime(p) for p in sorted(runtime.glob("*.dll"))}
    with tempfile.TemporaryDirectory(prefix="rexafs-installer-") as temp:
        staged = Path(temp) / "payload"
        shutil.copytree(bundle, staged)
        for name in runtime_files:
            if (staged / name).exists():
                raise ValueError(
                    f"Runtime would replace an existing payload file: {name}"
                )
            shutil.copy2(runtime / name, staged / name)
        (staged / "MICROSOFT-RUNTIME-NOTICE.txt").write_text(
            "Microsoft Visual C++ runtime DLLs are distributed beside rexafs for local deployment.\n"
            "Copyright Microsoft Corporation. These are unmodified, Microsoft-signed x64 DLLs\n"
            "from the Visual Studio redistributable directory. Applicable Microsoft terms:\n"
            "https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files\n"
            "Installed DLL versions and hashes are recorded in the installer build metadata.\n",
            encoding="utf-8",
        )
        payload = {
            p.relative_to(staged).as_posix(): digest(p)
            for p in sorted(staged.rglob("*"))
            if p.is_file()
        }
        defines = {
            "BundleDir": str(staged),
            "OutputDir": str(output),
            "OutputName": output_name(metadata),
            "AppVersion": metadata["version"],
            "FileVersion": metadata["version"].split("-")[0] + ".0",
            "Channel": metadata["channel"],
        }
        subprocess.run(
            [
                str(compiler),
                "/Qp",
                *[f"/D{k}={v}" for k, v in defines.items()],
                str(Path(__file__).with_name("windows-installer.iss")),
            ],
            check=True,
        )
    installer_digest = digest(installer)
    Path(str(installer) + ".sha256").write_text(
        f"{installer_digest}  {installer.name}\n", encoding="utf-8"
    )
    record = {
        "installer": installer.name,
        "sha256": installer_digest,
        "signed": False,
        "source_build": metadata,
        "payload_sha256": payload,
        "microsoft_runtime": runtime_files,
        "packaging_commit": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        "packaging_run_id": os.environ.get("GITHUB_RUN_ID"),
        "compiler": subprocess.run(
            [str(compiler), "/?"], capture_output=True, text=True, check=False
        ).stdout.splitlines()[:5],
    }
    installer.with_suffix(".build.json").write_text(
        json.dumps(record, indent=2) + "\n", encoding="utf-8"
    )
    return installer


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--bundle", type=Path)
    source.add_argument("--release")
    args = parser.parse_args()
    if args.release:
        with tempfile.TemporaryDirectory(prefix="rexafs-windows-source-") as temp:
            bundle = download_release(args.release, Path(temp) / "download")
            print(build(bundle, args.output))
    else:
        matches = list(args.bundle.parent.glob(args.bundle.name))
        if len(matches) != 1:
            raise ValueError("Expected exactly one Windows desktop bundle")
        print(build(matches[0], args.output))


if __name__ == "__main__":
    main()
