"""Cross-platform tests of Windows installer source validation and identity."""

import json
import tempfile
import unittest
from pathlib import Path
from zipfile import ZipFile, ZipInfo

from windows_installer import (
    TARGET,
    bundle_identity,
    digest,
    extract_release,
    output_name,
)

COMMIT = "a" * 40


def fixture(root: Path) -> Path:
    bundle = root / f"rexafs-0.1.3-{TARGET}"
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
    (bundle / "build.json").write_text(
        json.dumps(
            {
                "version": "0.1.3",
                "target": TARGET,
                "commit": COMMIT,
                "channel": "stable",
                "release_tag": "v0.1.3",
                "dirty": False,
                "features": ["refeff-runner"],
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


class InstallerTests(unittest.TestCase):
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
