"""Build an unsigned, per-user Windows installer from a qualified desktop bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any
from zipfile import ZipFile

from release_archive import engine_execution, validate_pe_binary

TARGET = "x86_64-pc-windows-msvc"
ARM64_TARGET = "aarch64-pc-windows-msvc"
TARGETS = {
    TARGET: {"architecture": "x64", "machine": 0x8664},
    ARM64_TARGET: {"architecture": "arm64", "machine": 0xAA64},
}
REPOSITORY = "Ameyanagi/rexafs"
VERSION = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?")
RUNTIME_NOTICE = "MICROSOFT-RUNTIME-NOTICE.txt"


def digest(path: Path) -> str:
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def pe_machine(path: Path) -> int:
    """Read the PE/COFF machine identifier without executing the file.

    Reject missing or truncated headers. This verifies the recorded CPU type,
    not executable integrity or runtime behavior. Header layout and identifiers:
    https://learn.microsoft.com/windows/win32/debug/pe-format#file-headers
    """
    with path.open("rb") as stream:
        header = stream.read(64)
        if len(header) != 64 or header[:2] != b"MZ":
            raise ValueError(f"Invalid PE header: {path}")
        offset = int.from_bytes(header[60:64], "little")
        if offset < 64:
            raise ValueError(f"Invalid PE header offset: {path}")
        stream.seek(offset)
        coff = stream.read(24)
        if len(coff) != 24 or coff[:4] != b"PE\x00\x00":
            raise ValueError(f"Invalid PE signature or truncated COFF header: {path}")
        return int.from_bytes(coff[4:6], "little")


def require_machine(
    path: Path, target: str, *, runtime: bool = False, gui: bool = False
) -> int:
    """Require a target's PE machine type, allowing ARM64X for ARM64 runtime DLLs.

    ARM64X DLLs contain native ARM64 code as well as ARM64EC code:
    https://learn.microsoft.com/windows/arm/arm64x-pe
    rexafs.exe itself must be a conventional native ARM64 or x64 executable.
    """
    expected = {TARGETS[target]["machine"]}
    if runtime and target == ARM64_TARGET:
        expected.add(0xA64E)
    machine = pe_machine(path)
    if machine not in expected:
        raise ValueError(
            f"Wrong PE machine for {target}: {path.name} is 0x{machine:04X}"
        )
    validate_pe_binary(path, machine, gui=gui)
    return machine


def pe_imports(path: Path) -> set[str]:
    """Read normal and delay-loaded DLL names from the default PE32+ image view.

    Decode relative virtual addresses through the section table; reject truncated
    tables and unsupported legacy delay descriptors rather than ignoring them.
    ARM64X's default view is native ARM64. This does not enumerate DLLs loaded
    dynamically by application code or substitute for native runtime tests.
    https://learn.microsoft.com/windows/win32/debug/pe-format
    https://learn.microsoft.com/windows/arm/arm64x-pe
    """
    validate_pe_binary(path, pe_machine(path))
    data = path.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    optional = pe + 24
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    section_count = struct.unpack_from("<H", data, pe + 6)[0]
    section_table = optional + optional_size
    if section_table + section_count * 40 > len(data):
        raise ValueError(f"Truncated PE section table: {path.name}")
    sections = [
        struct.unpack_from("<IIII", data, section_table + index * 40 + 8)
        for index in range(section_count)
    ]
    header_size = struct.unpack_from("<I", data, optional + 60)[0]

    def read_rva(rva: int, size: int) -> bytes:
        if 0 <= rva < header_size and rva + size <= min(header_size, len(data)):
            return data[rva : rva + size]
        for _virtual_size, address, raw_size, offset in sections:
            relative = rva - address
            if 0 <= relative and relative + size <= raw_size:
                start = offset + relative
                if start + size <= len(data):
                    return data[start : start + size]
        raise ValueError(f"Invalid PE import address in {path.name}: 0x{rva:X}")

    count = struct.unpack_from("<I", data, optional + 108)[0]
    if 112 + count * 8 > optional_size:
        raise ValueError(f"Truncated PE data directories: {path.name}")
    imports = set()
    for index, width, name_offset in [(1, 20, 12), (13, 32, 4)]:
        if index >= count:
            continue
        address, size = struct.unpack_from("<II", data, optional + 112 + index * 8)
        if not address and not size:
            continue
        if not address or size < width:
            raise ValueError(f"Invalid PE import directory: {path.name}")
        for offset in range(0, size - width + 1, width):
            entry = read_rva(address + offset, width)
            if not any(entry):
                break
            if index == 13 and struct.unpack_from("<I", entry)[0] != 1:
                raise ValueError(f"Unsupported PE delay import attributes: {path.name}")
            name_rva = struct.unpack_from("<I", entry, name_offset)[0]
            name = bytearray()
            for character in range(256):
                value = read_rva(name_rva + character, 1)
                if value == b"\0":
                    break
                name.extend(value)
            else:
                raise ValueError(f"Unterminated PE import name: {path.name}")
            decoded = name.decode("ascii").lower()
            if not re.fullmatch(r"[a-z0-9_.-]+\.dll", decoded):
                raise ValueError(f"Invalid PE import name in {path.name}: {decoded!r}")
            imports.add(decoded)
        else:
            raise ValueError(f"Unterminated PE import directory: {path.name}")
    return imports


def runtime_payload(bundle: Path, target: str, runtime: Path) -> list[Path]:
    """Select compatible CRT DLLs and require every static CRT import to be present.

    The ARM64 redistributable can contain the x64-only vcruntime140_1.dll FH4
    companion. Omit that one file only when neither the native desktop nor any
    selected native/ARM64X runtime imports it, including delay imports. Other
    machine mismatches remain errors. Microsoft describes FH4's x64 DLL here:
    https://devblogs.microsoft.com/cppblog/making-cpp-exception-handling-smaller-x64/
    """
    executable = bundle / "rexafs.exe"
    require_machine(executable, target)
    files, omitted = [], []
    for path in sorted(runtime.glob("*.dll")):
        if (
            target == ARM64_TARGET
            and path.name.lower() == "vcruntime140_1.dll"
            and pe_machine(path) == 0x8664
        ):
            validate_pe_binary(path, 0x8664)
            omitted.append(path.name)
            continue
        require_machine(path, target, runtime=True)
        files.append(path)
    available = {path.name.lower() for path in files}
    required_by = {}
    for path in [executable, *files]:
        required = {
            name
            for name in pe_imports(path)
            if re.fullmatch(r"(?:vcruntime|msvcp|concrt)[0-9][a-z0-9_.-]*\.dll", name)
        }
        required_by[path.name] = sorted(required)
        if missing := required - available:
            raise ValueError(
                f"{path.name} imports CRT DLLs unavailable for {target}: {', '.join(sorted(missing))}"
            )
    if omitted:
        print(f"Omitted unused x64 CRT companion: {', '.join(omitted)}")
        print("Verified native CRT imports: " + json.dumps(required_by, sort_keys=True))
    return files


def installer_policy(target: str) -> dict[str, str]:
    """Return architecture admission and Windows baseline for a desktop target.

    ARM64 packages use a native desktop and an x64 FEFF10 helper, requiring
    Windows 11 x64 emulation. Preserve the existing x64 installer policy.
    https://jrsoftware.org/ishelp/topic_setup_architecturesallowed.htm
    https://learn.microsoft.com/windows/arm/apps-on-arm-x86-emulation
    """
    if target not in TARGETS:
        raise ValueError(f"Unsupported Windows installer target: {target}")
    return {
        "architectures_allowed": (
            "arm64 and x64compatible" if target == ARM64_TARGET else "x64compatible"
        ),
        "minimum_windows_version": (
            "10.0.22000" if target == ARM64_TARGET else "10.0.19041"
        ),
    }


def bundle_identity(bundle: Path, target: str | None = None) -> dict[str, Any]:
    """Return metadata for a clean native Windows MSVC desktop bundle.

    Validate stable/nightly tag identity, source commit, example and notice
    files, executable architecture, and reject symlinks. An optional target must
    match the bundle; otherwise infer it from build.json. This checks identity;
    callers obtaining a ZIP from GitHub use extract_release to verify its
    checksum and expected source commit first. Mismatches raise ValueError.
    """
    metadata = json.loads((bundle / "build.json").read_text(encoding="utf-8"))
    version = metadata.get("version", "")
    if not isinstance(version, str) or not VERSION.fullmatch(version):
        raise ValueError("Invalid desktop version")
    bundle_target = metadata.get("target")
    if bundle_target not in TARGETS or metadata.get("dirty") is not False:
        raise ValueError("Installer requires a clean Windows MSVC desktop build")
    if target is not None and bundle_target != target:
        raise ValueError("Desktop bundle does not match the requested target")
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
    # Older qualified x64 releases used the console subsystem. Check their CPU
    # type without imposing the current desktop packaging's GUI-only policy.
    require_machine(bundle / "rexafs.exe", bundle_target)
    if "feff10-runner" in metadata.get("features", []):
        from feff10_worker import ASSETS

        for name, _ in ASSETS.values():
            helper_file = bundle / "resources/feff10" / name
            if not helper_file.is_file():
                raise ValueError(f"Missing FEFF10 helper payload: {name}")
            require_machine(helper_file, TARGET)
    return metadata


def output_name(metadata: dict[str, Any]) -> str:
    """Return the installer stem for already-validated bundle metadata.

    Stable uses the package version; nightly uses the dated release tag so
    different nightly runs produce distinct installer names. No files are made.
    """
    label = (
        metadata["release_tag"]
        if metadata["channel"] == "nightly"
        else metadata["version"]
    )
    return f"rexafs-{label}-{metadata['target']}-setup"


def extract_release(
    archive: Path,
    checksum: Path,
    destination: Path,
    tag: str,
    commit: str,
    target: str = TARGET,
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
    metadata = bundle_identity(bundle, target)
    if metadata["release_tag"] != tag or metadata["commit"] != commit:
        raise ValueError("Desktop ZIP is not from the requested release tag commit")
    return bundle


def download_release(tag: str, destination: Path, target: str = TARGET) -> Path:
    """Download a verified desktop ZIP; default to the historical x64 asset."""
    if target not in TARGETS:
        raise ValueError(f"Unsupported Windows installer target: {target}")
    if not tag.startswith("v") or not VERSION.fullmatch(tag[1:]):
        raise ValueError("Use a version tag such as v0.1.3")
    destination.mkdir(parents=True, exist_ok=False)
    name = f"rexafs-{tag[1:]}-{target}.zip"
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
        target,
    )
    return bundle


def runtime_directory(target: str = TARGET) -> Path:
    """Locate the newest installed Microsoft CRT for the requested target."""
    architecture = TARGETS[target]["architecture"]
    component = "ARM64" if target == ARM64_TARGET else "x86.x64"
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
            f"Microsoft.VisualStudio.Component.VC.Tools.{component}",
            "-property",
            "installationPath",
        ],
        text=True,
    ).strip()
    if not installation:
        raise ValueError("Visual Studio C++ redistributable directory not found")
    candidates = list(
        (Path(installation) / "VC/Redist/MSVC").glob(
            f"*/{architecture}/Microsoft.VC*.CRT"
        )
    )
    if not candidates:
        raise ValueError(f"No {architecture} Microsoft CRT redistributable directory")
    return max(
        candidates, key=lambda p: tuple(int(n) for n in p.parents[1].name.split("."))
    )


def signed_runtime(path: Path, target: str = TARGET) -> dict[str, str]:
    machine = require_machine(path, target, runtime=True)
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
    return {**result, "sha256": digest(path), "pe_machine": f"0x{machine:04X}"}


def stage_runtime(
    bundle: Path, target: str, runtime: Path | None = None
) -> dict[str, dict[str, str]]:
    """Copy a verified Microsoft C++ runtime beside the desktop executable.

    Run on Windows with an existing bundle directory. By default, locate the
    target's installed Visual Studio redistributable directory; runtime can
    select another redistributable directory. Select native/compatible DLLs and
    verify their imports, Microsoft signatures and PE architecture before copying
    any file. An unused x64-only FH4 companion in ARM64 redists is omitted.
    Reject existing destination DLLs or notices rather than replace them. Return
    DLL versions, signer names, SHA-256 hashes and machine identifiers for metadata.
    """
    architecture = TARGETS[target]["architecture"]
    runtime = runtime or runtime_directory(target)
    if not bundle.is_dir():
        raise ValueError(f"Runtime staging requires a bundle directory: {bundle}")
    if not (runtime / "vcruntime140.dll").is_file():
        raise ValueError(f"Microsoft {architecture} vcruntime140.dll is required")
    files = runtime_payload(bundle, target, runtime)
    for name in [*(path.name for path in files), RUNTIME_NOTICE]:
        destination = bundle / name
        if destination.exists() or destination.is_symlink():
            raise ValueError(f"Runtime would replace an existing payload file: {name}")
    provenance = {path.name: signed_runtime(path, target) for path in files}
    for path in files:
        shutil.copy2(path, bundle / path.name)
    (bundle / RUNTIME_NOTICE).write_text(
        "Microsoft Visual C++ runtime DLLs are distributed beside rexafs for local deployment.\n"
        f"Copyright Microsoft Corporation. These are unmodified, Microsoft-signed {architecture}-compatible DLLs\n"
        "from the Visual Studio redistributable directory. Applicable Microsoft terms:\n"
        "https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files\n"
        "DLL versions and hashes are recorded in the package or installer build metadata.\n",
        encoding="utf-8",
    )
    return provenance


def bundled_runtime(
    bundle: Path, target: str, provenance: dict[str, dict[str, str]]
) -> dict[str, dict[str, str]]:
    """Verify an already bundled runtime without changing the payload.

    Require the notice and vcruntime140.dll, then recheck each recorded DLL's
    Microsoft signature, architecture and metadata. Missing files or differences
    raise ValueError. This keeps a new installer faithful to its source ZIP.
    """
    if not isinstance(provenance, dict) or "vcruntime140.dll" not in provenance:
        raise ValueError(
            "Bundled Microsoft runtime metadata is missing vcruntime140.dll"
        )
    if not (bundle / RUNTIME_NOTICE).is_file():
        raise ValueError("Missing bundled Microsoft runtime notice")
    verified = {}
    for name, expected in provenance.items():
        if not re.fullmatch(r"[A-Za-z0-9_.-]+\.dll", name):
            raise ValueError(f"Invalid bundled Microsoft runtime name: {name}")
        path = bundle / name
        if not path.is_file() or path.is_symlink():
            raise ValueError(f"Missing or linked bundled Microsoft runtime: {name}")
        actual = signed_runtime(path, target)
        if actual != expected:
            raise ValueError(f"Bundled Microsoft runtime metadata mismatch: {name}")
        verified[name] = actual
    return verified


def installer_compiler() -> Path:
    """Locate Inno Setup 6 on PATH or in either Windows program directory."""
    found = shutil.which("ISCC.exe")
    if found:
        return Path(found)
    for variable in ("ProgramFiles(x86)", "ProgramFiles"):
        if directory := os.environ.get(variable):
            candidate = Path(directory) / "Inno Setup 6/ISCC.exe"
            if candidate.is_file():
                return candidate
    raise ValueError("Inno Setup 6 compiler not found")


def build(
    bundle: Path,
    output: Path,
    runtime: Path | None = None,
    compiler: Path | None = None,
    target: str | None = None,
) -> Path:
    """Build an unsigned per-user installer and write its provenance sidecars.

    Run on Windows with an existing qualified bundle. Defaults locate Inno Setup
    6 on PATH or under ProgramFiles and the Microsoft redistributable runtime through
    runtime_directory for legacy ZIPs. Reuse and verify the recorded runtime in
    newer ZIPs. Copy the bundle into temporary staging, compile the installer,
    and record source and payload hashes. Existing installer output is rejected.
    Return the EXE path;
    installation/reinstallation/uninstallation smoke checks run separately.
    Infer the target from build.json unless supplied. ARM64 packages require
    Windows 11 and run their bundled x64 FEFF10 helper under emulation.
    """
    if os.name != "nt":
        raise ValueError("Compile Windows installers on Windows")
    bundle = bundle.resolve()
    metadata = bundle_identity(bundle, target)
    target = metadata["target"]
    architecture = TARGETS[target]["architecture"]
    policy = installer_policy(target)
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    name = output_name(metadata)
    installer = output / (name + ".exe")
    if installer.exists():
        raise ValueError("Installer output already exists")
    compiler = compiler or installer_compiler()
    with tempfile.TemporaryDirectory(prefix="rexafs-installer-") as temp:
        staged = Path(temp) / "payload"
        shutil.copytree(bundle, staged)
        if "microsoft_runtime" in metadata:
            if runtime is not None:
                raise ValueError(
                    "Cannot replace the Microsoft runtime recorded in a source ZIP"
                )
            runtime_files = bundled_runtime(
                staged, target, metadata["microsoft_runtime"]
            )
        else:
            runtime_files = stage_runtime(staged, target, runtime)
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
            "Target": target,
            "ArchitecturesAllowed": policy["architectures_allowed"],
            "MinVersion": policy["minimum_windows_version"],
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
        "installer_policy": policy,
        "desktop_pe_machine": f"0x{pe_machine(bundle / 'rexafs.exe'):04X}",
        "runtime_architecture": architecture,
        "engine_execution": {
            engine: execution
            for engine, execution in engine_execution(target).items()
            if f"{engine}-runner" in metadata.get("features", [])
        },
        "payload_sha256": payload,
        "microsoft_runtime": runtime_files,
        "packaging_commit": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        "packaging_run_id": os.environ.get("GITHUB_RUN_ID"),
        "compiler": subprocess.run(
            [str(compiler), "/?"], capture_output=True, text=True, check=False
        ).stdout.splitlines()[:12],
        "compiler_sha256": digest(compiler),
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
    parser.add_argument(
        "--target",
        choices=tuple(TARGETS),
        help="Expected bundle target; release downloads default to x86_64-pc-windows-msvc",
    )
    args = parser.parse_args()
    if args.release:
        with tempfile.TemporaryDirectory(prefix="rexafs-windows-source-") as temp:
            bundle = download_release(
                args.release, Path(temp) / "download", args.target or TARGET
            )
            print(build(bundle, args.output, target=args.target))
    else:
        matches = list(args.bundle.parent.glob(args.bundle.name))
        if len(matches) != 1:
            raise ValueError("Expected exactly one Windows desktop bundle")
        print(build(matches[0], args.output, target=args.target))


if __name__ == "__main__":
    main()
