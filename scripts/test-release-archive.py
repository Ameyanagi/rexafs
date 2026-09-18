"""Regressions for desktop architecture, engine metadata and archive contents."""
from contextlib import ExitStack
import hashlib
import json
import os
from pathlib import Path
import runpy
import struct
import tempfile
import unittest
from unittest.mock import patch
from zipfile import ZipFile

import feff10_worker
from release_archive import (
    DESKTOP_TARGETS,
    engine_execution,
    validate_desktop_binary,
    validate_desktop_target,
    validate_pe_binary,
    zip_bundle,
)


def pe_fixture(machine: int, subsystem: int = 2) -> bytes:
    """Construct a PE32+ header fixture without executable program contents."""
    data = bytearray(512)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 128)
    data[128:132] = b"PE\0\0"
    struct.pack_into("<H", data, 132, machine)
    struct.pack_into("<H", data, 148, 240)
    struct.pack_into("<H", data, 152, 0x20B)
    struct.pack_into("<H", data, 220, subsystem)
    return bytes(data)


def elf_fixture(machine: int, kind: int = 3) -> bytes:
    """Construct an ELF64 header fixture for an executable or shared object."""
    data = bytearray(64)
    data[:7] = b"\x7fELF\x02\x01\x01"
    struct.pack_into("<HHI", data, 16, kind, machine, 1)
    struct.pack_into("<H", data, 52, 64)
    return bytes(data)


def macho_fixture(cpu: int) -> bytes:
    """Construct a thin Mach-O 64 header fixture for a native executable."""
    return struct.pack("<8I", 0xFEEDFACF, cpu, 0, 2, 0, 0, 0, 0)


class DesktopIdentityTests(unittest.TestCase):
    def test_target_must_match_system_and_ci(self):
        for target, system in DESKTOP_TARGETS.items():
            with self.subTest(target=target):
                validate_desktop_target(target, system, target)
                with self.assertRaises(ValueError):
                    validate_desktop_target(target, "Unknown", target)
                with self.assertRaisesRegex(ValueError, "expected desktop target"):
                    validate_desktop_target(target, system, "another-target")
        with self.assertRaises(ValueError):
            validate_desktop_target("aarch64-unknown-linux-musl", "Linux")

    def test_release_architecture_is_read_from_file_not_its_name(self):
        formats = [
            ("pc-windows-msvc", pe_fixture(0x8664), pe_fixture(0xAA64)),
            ("unknown-linux-gnu", elf_fixture(62), elf_fixture(183)),
            ("apple-darwin", macho_fixture(0x01000007), macho_fixture(0x0100000C)),
        ]
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "rexafs"
            for platform, x64, arm64 in formats:
                for architecture, data in [("x86_64", x64), ("aarch64", arm64)]:
                    target = f"{architecture}-{platform}"
                    with self.subTest(target=target):
                        binary.write_bytes(data)
                        validate_desktop_binary(binary, target)
                        wrong = "aarch64" if architecture == "x86_64" else "x86_64"
                        with self.assertRaises(ValueError):
                            validate_desktop_binary(binary, f"{wrong}-{platform}")
                        binary.write_bytes(data[:24])
                        with self.assertRaises(ValueError):
                            validate_desktop_binary(binary, target)

    def test_linux_rejects_non_native_formats_and_non_executables(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "rexafs"
            header = elf_fixture(183)
            for data in [
                pe_fixture(0xAA64),
                header[:4] + b"\x01" + header[5:],  # ELF32 is not an ARM64 desktop.
                header[:5] + b"\x02" + header[6:],  # Big-endian is not this target.
                elf_fixture(183, kind=1),  # A relocatable object is not runnable.
            ]:
                with self.subTest(header=data[:20]):
                    binary.write_bytes(data)
                    with self.assertRaises(ValueError):
                        validate_desktop_binary(binary, "aarch64-unknown-linux-gnu")
            binary.write_bytes(elf_fixture(183, kind=2))
            validate_desktop_binary(binary, "aarch64-unknown-linux-gnu")

    def test_windows_desktop_requires_gui_but_helper_allows_console(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "rexafs.exe"
            binary.write_bytes(pe_fixture(0xAA64, subsystem=3))
            validate_pe_binary(binary, 0xAA64)
            with self.assertRaisesRegex(ValueError, "GUI subsystem"):
                validate_desktop_binary(binary, "aarch64-pc-windows-msvc")

    def test_windows_rejects_malformed_headers(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "rexafs.exe"
            for offset, replacement in [
                (0, b"XX"),
                (0x3C, struct.pack("<I", 1)),
                (128, b"XX"),
                (148, struct.pack("<H", 4096)),
                (148, struct.pack("<H", 32)),
                (152, struct.pack("<H", 0x10B)),  # PE32 is not PE32+.
            ]:
                with self.subTest(offset=offset, replacement=replacement):
                    data = bytearray(pe_fixture(0xAA64))
                    data[offset:offset + len(replacement)] = replacement
                    binary.write_bytes(data)
                    with self.assertRaises(ValueError):
                        validate_desktop_binary(binary, "aarch64-pc-windows-msvc")

    def test_windows_arm64_metadata_discloses_x64_helper_emulation(self):
        target = "aarch64-pc-windows-msvc"
        engines = engine_execution(target)
        self.assertEqual(engines["refeff"], {"mode": "native", "target": target})
        self.assertEqual(engines["feff10"], {
            "mode": "helper-process", "target": "x86_64-pc-windows-gnu",
            "runtime": "windows-x64-emulation", "minimum_os": "Windows 11",
        })
        self.assertEqual(engine_execution("x86_64-pc-windows-msvc")["feff10"]["runtime"], "native")
        for target, system in DESKTOP_TARGETS.items():
            if system != "Windows":
                self.assertEqual(engine_execution(target)["feff10"], {
                    "mode": "native-stage-workers", "target": target,
                })
        with self.assertRaises(ValueError):
            engine_execution("aarch64-unknown-linux-musl")

    def test_bundled_feff10_payload_requires_x64_executable_and_dlls(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory)
            with patch.object(feff10_worker, "download", return_value=pe_fixture(0x8664, subsystem=3)):
                executable = feff10_worker.install_helper(destination)
            self.assertEqual(executable.name, "feff10-rs.exe")
            self.assertEqual(
                {path.name for path in destination.iterdir()},
                {name for name, _ in feff10_worker.ASSETS.values()},
            )
            # Reject a native ARM64 DLL accidentally mixed into the x64 helper.
            def wrong_runtime(asset, checksum):
                return pe_fixture(0xAA64 if asset.endswith(".dll") else 0x8664)

            with patch.object(feff10_worker, "download", side_effect=wrong_runtime):
                with self.assertRaisesRegex(ValueError, "PE machine"):
                    feff10_worker.install_helper(destination)


class ZipBundleTests(unittest.TestCase):
    def test_old_notice_dates_preserve_bytes_and_bundle_layout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = root / "rexafs-test-windows"
            notices = bundle / "licenses" / "example-1.0"
            notices.mkdir(parents=True)
            empty = bundle / "resources" / "empty"
            empty.mkdir(parents=True)
            original = b"License fixture\r\n\x00original bytes\n"
            notice = notices / "LICENSE"
            notice.write_bytes(original)
            # This reproduces the timestamp that broke the GitHub Windows ZIP.
            os.utime(notice, (86400, 86400))
            os.utime(empty, (86400, 86400))
            before = notice.stat().st_mtime_ns
            archive = root / "rexafs-test-windows.zip"
            zip_bundle(bundle, archive)
            with ZipFile(archive) as result:
                self.assertIsNone(result.testzip())
                name = "rexafs-test-windows/licenses/example-1.0/LICENSE"
                self.assertEqual(result.read(name), original)
                self.assertEqual(result.getinfo(name).date_time, (1980, 1, 1, 0, 0, 0))
                self.assertIn("rexafs-test-windows/resources/empty/", result.namelist())
                self.assertTrue(all(n.startswith("rexafs-test-windows/") for n in result.namelist()))
                result.extractall(root / "unpacked")
            self.assertEqual((root / "unpacked" / name).read_bytes(), original)
            self.assertEqual(notice.stat().st_mtime_ns, before)


class WindowsArchiveRuntimeTests(unittest.TestCase):
    def test_runtime_is_present_before_execution_and_retained_in_zip(self):
        # Exercise packaging and extraction on both targets. Only external
        # processes, signatures and downloads are mocked; the archive is real.
        script = Path(__file__).with_name("package-desktop.py").read_text()
        for target, machine in [
            ("x86_64-pc-windows-msvc", 0x8664),
            ("aarch64-pc-windows-msvc", 0xAA64),
        ]:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                package_script = root / "scripts/package-desktop.py"
                package_script.parent.mkdir()
                package_script.write_text(script)
                fixtures = [
                    "assets/brand/rexafs-icon.png", "assets/brand/rexafs.ico",
                    "crates/rexafs/tests/testfiles/xraylarch_d867/xafsdata/cu_150k.xmu",
                    "crates/rexafs/tests/testfiles/xraylarch_d867/README.md",
                    "crates/rexafs/data/atomic/XrayDB-LICENSE.txt",
                    "assets/licenses/feff10-native/NOTICE", "LICENSE-MIT", "LICENSE-APACHE",
                ]
                for name in fixtures:
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(b"fixture")
                executable = root / "target/release/rexafs.exe"
                executable.parent.mkdir(parents=True)
                executable.write_bytes(pe_fixture(machine))
                runtime = root / "redist"
                runtime.mkdir()
                runtime_bytes = pe_fixture(machine, subsystem=3)
                (runtime / "vcruntime140.dll").write_bytes(runtime_bytes)
                provenance = {
                    "version": "14.50.test", "signer": "O=Microsoft Corporation",
                    "sha256": hashlib.sha256(runtime_bytes).hexdigest(),
                    "pe_machine": f"0x{machine:04X}",
                }
                build = {
                    "version": "0.2.5", "channel": "stable", "release_tag": "v0.2.5",
                    "built_at": None, "features": ["refeff-runner", "feff10-runner"],
                }
                cargo = {
                    "packages": [{
                        "id": "gui", "name": "rexafs-gui", "version": "0.2.5",
                        "manifest_path": str(root / "crates/rexafs-gui/Cargo.toml"),
                    }],
                    "resolve": {"nodes": [{"id": "gui", "deps": []}]},
                    "target_directory": str(root / "target"),
                }

                def command_output(command, **kwargs):
                    if command[:2] == ["cargo", "metadata"]:
                        return json.dumps(cargo)
                    if command == ["rustc", "-vV"]:
                        return f"rustc 1.98.1\nhost: {target}\n"
                    if command == ["rustc", "--version"]:
                        return "rustc 1.98.1\n"
                    if command[:2] == ["git", "rev-parse"]:
                        return "a" * 40 + "\n"
                    if command[:2] == ["git", "status"]:
                        return b""
                    self.assertEqual(command[1:], ["--build-info"])
                    self.assertEqual(
                        (Path(command[0]).parent / "vcruntime140.dll").read_bytes(), runtime_bytes,
                    )
                    return json.dumps(build)

                checked = []

                def check_extracted(command, **kwargs):
                    extracted = Path(command[0]).parent
                    self.assertEqual((extracted / "vcruntime140.dll").read_bytes(), runtime_bytes)
                    self.assertTrue((extracted / "MICROSOFT-RUNTIME-NOTICE.txt").is_file())
                    self.assertNotIn("REXAFS_FEFF10_EXECUTABLE", kwargs["env"])
                    checked.append(command[1])

                with ExitStack() as patches:
                    patches.enter_context(patch("platform.system", return_value="Windows"))
                    patches.enter_context(patch.dict(os.environ, {
                        "REXAFS_EXPECTED_TARGET": target,
                        "REXAFS_FEFF10_EXECUTABLE": "source-tree-helper.exe",
                    }, clear=True))
                    patches.enter_context(patch("subprocess.check_output", side_effect=command_output))
                    patches.enter_context(patch("subprocess.run", side_effect=check_extracted))
                    patches.enter_context(patch("windows_installer.runtime_directory", return_value=runtime))
                    patches.enter_context(patch("windows_installer.signed_runtime", return_value=provenance))
                    patches.enter_context(patch.object(feff10_worker, "download", return_value=pe_fixture(0x8664)))
                    patches.enter_context(patch("builtins.print"))
                    runpy.run_path(str(package_script), run_name="__main__")
                self.assertEqual(checked, ["--version", "--self-check", "--self-check-feff"])
                stem = f"rexafs-0.2.5-{target}"
                with ZipFile(root / f"target/distributions/{stem}.zip") as archive:
                    self.assertEqual(archive.read(stem + "/vcruntime140.dll"), runtime_bytes)
                    self.assertEqual(
                        archive.read(stem + "/licenses/rexafs-atomic-data/XrayDB-LICENSE.txt"),
                        b"fixture",
                    )
                    metadata = json.loads(archive.read(stem + "/build.json"))
                    self.assertEqual(metadata["microsoft_runtime"], {"vcruntime140.dll": provenance})
                    self.assertEqual(metadata["target"], target)


if __name__ == "__main__":
    unittest.main()
