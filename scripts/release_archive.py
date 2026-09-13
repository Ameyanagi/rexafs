"""Native desktop identity and archive checks shared by release packaging."""
from __future__ import annotations

from pathlib import Path
import struct
from zipfile import ZIP_DEFLATED, ZipFile


DESKTOP_TARGETS = {
    "aarch64-apple-darwin": "Darwin",
    "x86_64-apple-darwin": "Darwin",
    "aarch64-pc-windows-msvc": "Windows",
    "x86_64-pc-windows-msvc": "Windows",
    "aarch64-unknown-linux-gnu": "Linux",
    "x86_64-unknown-linux-gnu": "Linux",
}


def validate_desktop_target(target: str, system: str, expected: str | None = None) -> None:
    """Require a supported native Rust host matching the CI target, when supplied.

    This check rejects a different toolchain host before packaging starts. The
    executable header is checked separately by validate_desktop_binary.
    """
    if DESKTOP_TARGETS.get(target) != system:
        raise ValueError(f"Unsupported desktop target {target!r} on {system}")
    if expected and target != expected:
        raise ValueError(f"Rust host {target} does not match the expected desktop target {expected}")


def validate_pe_binary(binary: Path, machine: int, *, gui: bool = False) -> None:
    """Require a 64-bit Windows Portable Executable (PE) with the given machine value.

    The machine value comes from its Common Object File Format (COFF) header.
    Set gui=True for the desktop executable: its subsystem must be Windows GUI
    (2), so launching it does not open a console. Helpers and DLLs do not need
    that subsystem. Missing, truncated or mismatched headers raise ValueError.
    This checks file identity; it does not establish runtime compatibility.
    """
    with binary.open("rb") as executable:
        dos = executable.read(64)
        if len(dos) != 64 or dos[:2] != b"MZ":
            raise ValueError(f"{binary.name}: missing or truncated DOS header")
        pe_offset = struct.unpack_from("<I", dos, 0x3C)[0]
        if pe_offset < 64:
            raise ValueError(f"{binary.name}: invalid PE header offset")
        executable.seek(pe_offset)
        coff = executable.read(24)
        if len(coff) != 24 or coff[:4] != b"PE\0\0":
            raise ValueError(f"{binary.name}: missing or truncated PE header")
        actual = struct.unpack_from("<H", coff, 4)[0]
        if actual != machine:
            raise ValueError(f"{binary.name}: PE machine 0x{actual:04x} does not match 0x{machine:04x}")
        optional_size = struct.unpack_from("<H", coff, 20)[0]
        optional = executable.read(optional_size)
        if len(optional) != optional_size or optional_size < 112 or optional[:2] != b"\x0b\x02":
            raise ValueError(f"{binary.name}: missing or truncated PE32+ optional header")
        if gui and struct.unpack_from("<H", optional, 68)[0] != 2:
            raise ValueError("Windows desktop must use the GUI subsystem (no console window)")


def validate_desktop_binary(binary: Path, target: str) -> None:
    """Require the desktop's executable format and architecture to match its target.

    Linux requires a little-endian ELF64 executable or position-independent
    executable. Windows requires PE32+ with the GUI subsystem. macOS requires
    a thin, little-endian Mach-O 64 executable, as produced by the native Cargo
    release build. Header mismatches raise ValueError before anything is run.
    """
    system = DESKTOP_TARGETS.get(target)
    if system is None:
        raise ValueError(f"Unsupported desktop target: {target}")
    arm64 = target.startswith("aarch64-")
    if system == "Windows":
        validate_pe_binary(binary, 0xAA64 if arm64 else 0x8664, gui=True)
        return
    with binary.open("rb") as executable:
        header = executable.read(64)
    if system == "Linux":
        if len(header) != 64 or header[:7] != b"\x7fELF\x02\x01\x01":
            raise ValueError(f"{binary.name}: expected a little-endian ELF64 header")
        kind, actual, version = struct.unpack_from("<HHI", header, 16)
        expected = 183 if arm64 else 62
        if actual != expected:
            raise ValueError(f"{binary.name}: ELF machine {actual} does not match {expected}")
        if kind not in {2, 3} or version != 1 or struct.unpack_from("<H", header, 52)[0] != 64:
            raise ValueError(f"{binary.name}: invalid ELF64 executable header")
    else:
        if len(header) < 32 or header[:4] != b"\xcf\xfa\xed\xfe":
            raise ValueError(f"{binary.name}: expected a thin little-endian Mach-O 64 header")
        actual = struct.unpack_from("<I", header, 4)[0]
        expected = 0x0100000C if arm64 else 0x01000007
        if actual != expected or struct.unpack_from("<I", header, 12)[0] != 2:
            raise ValueError(f"{binary.name}: Mach-O architecture or executable type does not match {target}")


def engine_execution(target: str) -> dict[str, dict[str, str]]:
    """Describe how the two required desktop engines run on a supported target.

    These are package requirements, not a runtime qualification result. Windows
    ARM64 runs rexafs and ReFEFF natively, and the pinned x64 FEFF10 helper in a
    separate emulated process. x64 emulation requires Windows 11 on Arm; see
    https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-x86-emulation.
    """
    if target not in DESKTOP_TARGETS:
        raise ValueError(f"Unsupported desktop target: {target}")
    feff10 = {"mode": "native-stage-workers", "target": target}
    if DESKTOP_TARGETS[target] == "Windows":
        feff10 = {"mode": "helper-process", "target": "x86_64-pc-windows-gnu", "runtime": "native"}
        if target.startswith("aarch64-"):
            feff10.update(runtime="windows-x64-emulation", minimum_os="Windows 11")
    return {"refeff": {"mode": "native", "target": target}, "feff10": feff10}


def zip_bundle(bundle: Path, archive: Path) -> None:
    """Keep the bundle root and bytes, clamping only unsupported ZIP dates.

    Cargo packages can contain epoch-dated license files. ZIP dates only cover
    1980 through 2107; strict_timestamps=False handles both limits without
    changing source files or their timestamps.
    """
    with ZipFile(archive, "w", compression=ZIP_DEFLATED, strict_timestamps=False) as output:
        for source in [bundle, *sorted(bundle.rglob("*"))]:
            output.write(source, source.relative_to(bundle.parent).as_posix())
