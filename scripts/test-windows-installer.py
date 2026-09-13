"""Cross-platform tests of Windows installer source validation and identity."""

import json
import os
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from zipfile import ZipFile, ZipInfo

from windows_installer import (
    ARM64_TARGET,
    RUNTIME_NOTICE,
    TARGET,
    TARGETS,
    build,
    bundle_identity,
    bundled_runtime,
    digest,
    download_release,
    extract_release,
    installer_compiler,
    installer_policy,
    output_name,
    pe_machine,
    require_machine,
    runtime_directory,
    signed_runtime,
    stage_runtime,
)

COMMIT = "a" * 40


def pe_fixture(machine: int, subsystem: int = 2) -> bytes:
    """Make a minimal PE32+ header; no executable instructions are included."""
    data = bytearray(512)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<H", data, 0x84, machine)
    struct.pack_into("<H", data, 0x94, 0xF0)
    struct.pack_into("<H", data, 0x96, 0x22)
    struct.pack_into("<H", data, 0x98, 0x20B)
    struct.pack_into("<H", data, 0x98 + 68, subsystem)
    return bytes(data)


def fixture(root: Path, target: str = TARGET, feff10: bool = False) -> Path:
    bundle = root / f"rexafs-0.1.3-{target}"
    files = [
        "rexafs.exe",
        "resources/rexafs.ico",
        "resources/examples/cu_150k.xmu",
        "resources/examples/PROVENANCE.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "dependencies.json",
        "licenses/example/LICENSE",
    ]
    for name in files:
        path = bundle / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"test payload\x00\xff")
    (bundle / "rexafs.exe").write_bytes(pe_fixture(int(TARGETS[target]["machine"])))
    if feff10:
        from feff10_worker import ASSETS

        for name, _ in ASSETS.values():
            path = bundle / "resources/feff10" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(pe_fixture(0x8664, subsystem=3))
    (bundle / "build.json").write_text(
        json.dumps(
            {
                "version": "0.1.3",
                "target": target,
                "commit": COMMIT,
                "channel": "stable",
                "release_tag": "v0.1.3",
                "dirty": False,
                "features": ["refeff-runner", "feff10-runner"]
                if feff10
                else ["refeff-runner"],
            }
        )
    )
    return bundle


def archive_bundle(bundle: Path) -> tuple[Path, Path]:
    archive = Path(str(bundle) + ".zip")
    with ZipFile(archive, "w") as output:
        for path in sorted(bundle.rglob("*")):
            if path.is_file():
                output.write(path, path.relative_to(bundle.parent).as_posix())
    checksum = Path(str(archive) + ".sha256")
    checksum.write_text(f"{digest(archive)}  {archive.name}\n")
    return archive, checksum


def runtime_fixture(root: Path, machine: int) -> Path:
    runtime = root / "runtime"
    runtime.mkdir()
    for name in ["vcruntime140.dll", "msvcp140.dll"]:
        (runtime / name).write_bytes(pe_fixture(machine))
    return runtime


def runtime_signature_stub(path: Path, target: str) -> dict[str, str]:
    """Replace Windows signature lookup while retaining real PE and hash checks."""
    machine = require_machine(path, target, runtime=True)
    return {
        "version": "14.44.0.0",
        "signer": "O=Microsoft Corporation",
        "sha256": digest(path),
        "pe_machine": f"0x{machine:04X}",
    }


class InstallerTests(unittest.TestCase):
    def test_stage_runtime_copies_native_crt_and_records_its_provenance(self):
        for target in [TARGET, ARM64_TARGET]:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                bundle = fixture(root, target)
                executable = (bundle / "rexafs.exe").read_bytes()
                runtime = runtime_fixture(root, int(TARGETS[target]["machine"]))
                with patch(
                    "windows_installer.signed_runtime",
                    side_effect=runtime_signature_stub,
                ) as verify:
                    provenance = stage_runtime(bundle, target, runtime)
                    self.assertEqual(verify.call_count, 2)
                    for name, recorded in provenance.items():
                        self.assertEqual(
                            (bundle / name).read_bytes(), (runtime / name).read_bytes()
                        )
                        self.assertEqual(recorded["sha256"], digest(bundle / name))
                    before = {
                        p.name: p.read_bytes() for p in bundle.iterdir() if p.is_file()
                    }
                    self.assertEqual(
                        bundled_runtime(bundle, target, provenance), provenance
                    )
                    self.assertEqual(
                        before,
                        {
                            p.name: p.read_bytes()
                            for p in bundle.iterdir()
                            if p.is_file()
                        },
                    )
                self.assertEqual((bundle / "rexafs.exe").read_bytes(), executable)
                notice = (bundle / RUNTIME_NOTICE).read_text()
                self.assertIn(f"{TARGETS[target]['architecture']}-compatible", notice)
                self.assertFalse((runtime / RUNTIME_NOTICE).exists())

    def test_stage_runtime_never_replaces_payload_files_or_notices(self):
        for existing in ["vcruntime140.dll", RUNTIME_NOTICE]:
            with self.subTest(existing=existing), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                bundle = fixture(root)
                runtime = runtime_fixture(root, 0x8664)
                (bundle / existing).write_bytes(b"keep existing payload")
                with patch("windows_installer.signed_runtime") as verify:
                    with self.assertRaisesRegex(
                        ValueError, "replace an existing payload"
                    ):
                        stage_runtime(bundle, TARGET, runtime)
                    verify.assert_not_called()
                self.assertEqual(
                    (bundle / existing).read_bytes(), b"keep existing payload"
                )
                self.assertFalse((bundle / "msvcp140.dll").exists())

    def test_stage_runtime_verifies_all_signatures_before_copying(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = fixture(root)
            runtime = runtime_fixture(root, 0x8664)
            with (
                patch(
                    "windows_installer.signed_runtime",
                    side_effect=[{}, ValueError("bad signature")],
                ),
                self.assertRaisesRegex(ValueError, "bad signature"),
            ):
                stage_runtime(bundle, TARGET, runtime)
            self.assertFalse(any(bundle.glob("*.dll")))
            self.assertFalse((bundle / RUNTIME_NOTICE).exists())

    def test_bundled_runtime_rejects_changed_bytes_or_missing_notice(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = fixture(root, ARM64_TARGET)
            runtime = runtime_fixture(root, 0xAA64)
            with patch(
                "windows_installer.signed_runtime", side_effect=runtime_signature_stub
            ):
                provenance = stage_runtime(bundle, ARM64_TARGET, runtime)
                library = bundle / "vcruntime140.dll"
                library.write_bytes(library.read_bytes() + b"changed")
                with self.assertRaisesRegex(ValueError, "metadata mismatch"):
                    bundled_runtime(bundle, ARM64_TARGET, provenance)
                (bundle / RUNTIME_NOTICE).unlink()
                with self.assertRaisesRegex(ValueError, "runtime notice"):
                    bundled_runtime(bundle, ARM64_TARGET, provenance)

    def test_installer_preserves_source_bundle_with_or_without_bundled_runtime(self):
        for recorded in [False, True]:
            with self.subTest(recorded=recorded), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                bundle = fixture(root, ARM64_TARGET)
                runtime = runtime_fixture(root, 0xAA64)
                compiler = root / "ISCC.exe"
                compiler.write_bytes(b"test compiler")
                with patch(
                    "windows_installer.signed_runtime",
                    side_effect=runtime_signature_stub,
                ):
                    if recorded:
                        metadata_path = bundle / "build.json"
                        metadata = json.loads(metadata_path.read_text())
                        metadata["microsoft_runtime"] = stage_runtime(
                            bundle, ARM64_TARGET, runtime
                        )
                        metadata_path.write_text(json.dumps(metadata))
                    original = {
                        p.relative_to(bundle): p.read_bytes()
                        for p in bundle.rglob("*")
                        if p.is_file()
                    }

                    def compile_stub(command, *, runtime=runtime, **_kwargs):
                        if "/Qp" in command:
                            definitions = dict(
                                argument[2:].split("=", 1)
                                for argument in command
                                if argument.startswith("/D")
                            )
                            staged = Path(definitions["BundleDir"])
                            self.assertEqual(
                                (staged / "vcruntime140.dll").read_bytes(),
                                (runtime / "vcruntime140.dll").read_bytes(),
                            )
                            output = Path(definitions["OutputDir"]) / (
                                definitions["OutputName"] + ".exe"
                            )
                            output.write_bytes(b"compiled installer")
                        return SimpleNamespace(stdout="test Inno Setup compiler")

                    with (
                        patch(
                            "windows_installer.os",
                            SimpleNamespace(name="nt", environ={}),
                        ),
                        patch(
                            "windows_installer.subprocess.run", side_effect=compile_stub
                        ),
                        patch(
                            "windows_installer.subprocess.check_output",
                            return_value=COMMIT,
                        ),
                        patch(
                            "windows_installer.runtime_directory",
                            side_effect=AssertionError("must not rediscover runtime"),
                        ),
                    ):
                        installer = build(
                            bundle,
                            root / "output",
                            runtime=None if recorded else runtime,
                            compiler=compiler,
                        )
                    result = json.loads(
                        installer.with_suffix(".build.json").read_text()
                    )
                    self.assertEqual(
                        result["microsoft_runtime"]["vcruntime140.dll"]["sha256"],
                        digest(runtime / "vcruntime140.dll"),
                    )
                    self.assertEqual(
                        original,
                        {
                            p.relative_to(bundle): p.read_bytes()
                            for p in bundle.rglob("*")
                            if p.is_file()
                        },
                    )

    def test_arm64_bundle_and_release_have_distinct_target_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = fixture(root, ARM64_TARGET, feff10=True)
            metadata = bundle_identity(bundle)
            self.assertEqual(
                output_name(metadata), f"rexafs-0.1.3-{ARM64_TARGET}-setup"
            )
            archive, checksum = archive_bundle(bundle)
            extract_release(
                archive, checksum, root / "arm64", "v0.1.3", COMMIT, ARM64_TARGET
            )
            with self.assertRaisesRegex(ValueError, "requested target"):
                extract_release(archive, checksum, root / "x64", "v0.1.3", COMMIT)

    def test_native_architecture_must_match_recorded_target(self):
        for target, wrong_machine in [(TARGET, 0xAA64), (ARM64_TARGET, 0x8664)]:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temp:
                bundle = fixture(Path(temp), target)
                executable = bundle / "rexafs.exe"
                executable.write_bytes(pe_fixture(wrong_machine))
                with self.assertRaisesRegex(ValueError, "Wrong PE machine"):
                    bundle_identity(bundle)

    def test_legacy_x64_console_subsystem_bundle_is_still_accepted(self):
        with tempfile.TemporaryDirectory() as temp:
            bundle = fixture(Path(temp))
            (bundle / "rexafs.exe").write_bytes(pe_fixture(0x8664, subsystem=3))
            self.assertEqual(bundle_identity(bundle)["target"], TARGET)

    def test_rejects_truncated_or_invalid_executable_headers(self):
        with tempfile.TemporaryDirectory() as temp:
            bundle = fixture(Path(temp))
            executable = bundle / "rexafs.exe"
            for data in [b"MZ", pe_fixture(0x8664)[:100], b"test" * 200]:
                executable.write_bytes(data)
                with (
                    self.subTest(size=len(data)),
                    self.assertRaisesRegex(ValueError, "PE"),
                ):
                    bundle_identity(bundle)

    def test_feff10_helper_remains_x64_in_arm64_package(self):
        with tempfile.TemporaryDirectory() as temp:
            bundle = fixture(Path(temp), ARM64_TARGET, feff10=True)
            bundle_identity(bundle)
            library = bundle / "resources/feff10/libgfortran-5.dll"
            library.write_bytes(pe_fixture(0xAA64))
            with self.assertRaisesRegex(ValueError, "Wrong PE machine"):
                bundle_identity(bundle)
            library.unlink()
            with self.assertRaisesRegex(ValueError, "Missing FEFF10 helper payload"):
                bundle_identity(bundle)

    def test_installer_policy_retains_x64_and_requires_arm64_windows_11(self):
        self.assertEqual(
            installer_policy(TARGET),
            {
                "architectures_allowed": "x64compatible",
                "minimum_windows_version": "10.0.19041",
            },
        )
        self.assertEqual(
            installer_policy(ARM64_TARGET),
            {
                "architectures_allowed": "arm64 and x64compatible",
                "minimum_windows_version": "10.0.22000",
            },
        )
        with self.assertRaisesRegex(ValueError, "Unsupported"):
            installer_policy("i686-pc-windows-msvc")

    def test_runtime_discovery_selects_target_architecture(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for architecture in ["x64", "arm64"]:
                for version in ["14.9.0", "14.44.0"]:
                    (
                        root
                        / "VC/Redist/MSVC"
                        / version
                        / architecture
                        / "Microsoft.VC143.CRT"
                    ).mkdir(parents=True)
            with (
                patch.dict(os.environ, {"ProgramFiles(x86)": temp}),
                patch(
                    "windows_installer.subprocess.check_output", return_value=str(root)
                ) as command,
            ):
                for target, architecture in [(TARGET, "x64"), (ARM64_TARGET, "arm64")]:
                    directory = runtime_directory(target)
                    self.assertEqual(directory.parent.name, architecture)
                    self.assertEqual(directory.parents[1].name, "14.44.0")
                self.assertIn(
                    "Microsoft.VisualStudio.Component.VC.Tools.ARM64",
                    command.call_args.args[0],
                )

    def test_runtime_dll_architecture_checked_before_signature(self):
        with tempfile.TemporaryDirectory() as temp:
            runtime = Path(temp) / "vcruntime140.dll"
            runtime.write_bytes(pe_fixture(0x8664))
            with patch("windows_installer.subprocess.check_output") as command:
                with self.assertRaisesRegex(ValueError, "Wrong PE machine"):
                    signed_runtime(runtime, ARM64_TARGET)
                command.assert_not_called()
            for machine in [0xAA64, 0xA64E]:
                runtime.write_bytes(pe_fixture(machine))
                self.assertEqual(
                    require_machine(runtime, ARM64_TARGET, runtime=True), machine
                )
                self.assertEqual(pe_machine(runtime), machine)
            runtime.write_bytes(pe_fixture(0xA641))
            with self.assertRaisesRegex(ValueError, "Wrong PE machine"):
                require_machine(runtime, ARM64_TARGET, runtime=True)

    def test_release_download_defaults_to_x64_and_can_select_arm64(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            with (
                patch("windows_installer.subprocess.run") as download,
                patch("windows_installer.subprocess.check_output", return_value=COMMIT),
                patch(
                    "windows_installer.extract_release", return_value=root
                ) as extract,
            ):
                for target in [TARGET, ARM64_TARGET]:
                    destination = root / target
                    if target == TARGET:
                        download_release("v0.1.3", destination)
                    else:
                        download_release("v0.1.3", destination, target)
                    self.assertIn(
                        f"rexafs-0.1.3-{target}.zip", download.call_args.args[0]
                    )
                    self.assertEqual(extract.call_args.args[-1], target)

    def test_compiler_lookup_supports_both_program_directories(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            native = root / "native/Inno Setup 6/ISCC.exe"
            native.parent.mkdir(parents=True)
            native.write_bytes(b"compiler")
            x86 = root / "x86/Inno Setup 6/ISCC.exe"
            with (
                patch.dict(
                    os.environ,
                    {
                        "ProgramFiles": str(root / "native"),
                        "ProgramFiles(x86)": str(root / "x86"),
                    },
                ),
                patch("windows_installer.shutil.which", return_value=None),
            ):
                self.assertEqual(installer_compiler(), native)
                x86.parent.mkdir(parents=True)
                x86.write_bytes(b"compiler")
                self.assertEqual(installer_compiler(), x86)

    def test_verified_release_preserves_every_payload_byte(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = fixture(root)
            archive, checksum = archive_bundle(bundle)
            extracted = extract_release(
                archive, checksum, root / "extracted", "v0.1.3", COMMIT
            )
            for path in bundle.rglob("*"):
                if path.is_file():
                    self.assertEqual(
                        path.read_bytes(),
                        (extracted / path.relative_to(bundle)).read_bytes(),
                    )

    def test_rejects_modified_archive_or_wrong_checksum_name(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive, checksum = archive_bundle(fixture(root))
            for contents in [
                f"{digest(archive)}  other.zip\n",
                f"{'0' * 64}  {archive.name}\n",
            ]:
                checksum.write_text(contents)
                with self.assertRaisesRegex(ValueError, "checksum"):
                    extract_release(
                        archive, checksum, root / "extracted", "v0.1.3", COMMIT
                    )
                self.assertFalse((root / "extracted").exists())

    def test_rejects_wrong_release_tag_and_commit(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive, checksum = archive_bundle(fixture(root))
            for tag, commit in [("v0.1.4", COMMIT), ("v0.1.3", "b" * 40)]:
                with self.assertRaisesRegex(ValueError, "release tag commit"):
                    extract_release(archive, checksum, root / "extracted", tag, commit)

    def test_rejects_unsafe_zip_paths_and_case_collisions(self):
        for name in [
            "../escape.exe",
            "/absolute.exe",
            "root\\escape.exe",
            "root/C:escape.exe",
            "root/trailing./file",
            "root/rexafs.exe",
        ]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                bundle = fixture(root)
                archive, checksum = archive_bundle(bundle)
                member = name.replace("root/", bundle.name + "/")
                if name == "root/rexafs.exe":
                    member = bundle.name + "/REXAFS.EXE"
                with ZipFile(archive, "a") as output:
                    output.writestr(member, "unsafe")
                checksum.write_text(f"{digest(archive)}  {archive.name}\n")
                with self.assertRaisesRegex(ValueError, "Unsafe|ambiguous"):
                    extract_release(
                        archive, checksum, root / "extracted", "v0.1.3", COMMIT
                    )
                self.assertFalse((root / "extracted").exists())

    def test_rejects_archive_symlink(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = fixture(root)
            archive, checksum = archive_bundle(bundle)
            entry = ZipInfo(bundle.name + "/link")
            entry.create_system = 3
            entry.external_attr = 0o120777 << 16
            with ZipFile(archive, "a") as output:
                output.writestr(entry, "../../outside")
            checksum.write_text(f"{digest(archive)}  {archive.name}\n")
            with self.assertRaisesRegex(ValueError, "Unsafe"):
                extract_release(archive, checksum, root / "extracted", "v0.1.3", COMMIT)

    def test_rejects_wrong_platform_dirty_build_and_missing_resources(self):
        with tempfile.TemporaryDirectory() as temp:
            bundle = fixture(Path(temp))
            path = bundle / "build.json"
            original = json.loads(path.read_text())
            for change in [
                {"target": "aarch64-apple-darwin"},
                {"dirty": True},
                {"commit": "short"},
                {"version": '0.1.3"\nInjected=yes'},
                {"release_tag": "v0.1.4"},
                {"channel": "unknown"},
            ]:
                path.write_text(json.dumps({**original, **change}))
                with self.subTest(change=change), self.assertRaises(ValueError):
                    bundle_identity(bundle)
            path.write_text(json.dumps(original))
            (bundle / "resources/rexafs.ico").unlink()
            with self.assertRaisesRegex(ValueError, "Missing installer payload"):
                bundle_identity(bundle)

    def test_channel_identities_have_distinct_immutable_names(self):
        with tempfile.TemporaryDirectory() as temp:
            bundle = fixture(Path(temp))
            metadata = bundle_identity(bundle)
            self.assertEqual(output_name(metadata), f"rexafs-0.1.3-{TARGET}-setup")
            metadata.update(
                channel="nightly",
                release_tag="nightly-20260908-123",
                built_at="2026-09-08T00:00:00Z",
            )
            (bundle / "build.json").write_text(json.dumps(metadata))
            self.assertEqual(
                output_name(bundle_identity(bundle)),
                f"rexafs-nightly-20260908-123-{TARGET}-setup",
            )
            metadata["built_at"] = None
            (bundle / "build.json").write_text(json.dumps(metadata))
            with self.assertRaisesRegex(ValueError, "timestamp"):
                bundle_identity(bundle)


if __name__ == "__main__":
    unittest.main()
