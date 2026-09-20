"""Keep registry packages off the desktop download list without weakening hashes."""
import hashlib
from pathlib import Path
import tempfile
import unittest

from release_downloads import stage, targets_for_version


class ReleaseDownloadsTests(unittest.TestCase):
    def fixture(self, root, version="0.2.0"):
        artifacts = root / "artifacts"
        artifacts.mkdir()
        names = [f"rexafs-{version}{suffix}" for suffix in
                 (".crate", ".tgz", ".tar.gz", "-cp312-cp312-win_amd64.whl")]
        for target, extension in targets_for_version(version).items():
            stem = f"rexafs-{version}-{target}"
            names.extend([stem + extension, stem + extension + ".sha256"])
            if target.endswith("windows-msvc"):
                names.extend(stem + suffix for suffix in ("-setup.exe", "-setup.exe.sha256", "-setup.build.json", "-setup.exe.qualification.json"))
            if target.endswith("apple-darwin"):
                names.extend(stem + suffix for suffix in (".dmg", ".dmg.sha256", ".dmg.json"))
        for name in names:
            (artifacts / name).write_text("Original " + name)
        manifest = artifacts / "SHA256SUMS"
        manifest.write_text("".join(f"{hashlib.sha256((artifacts/name).read_bytes()).hexdigest()}  {name}\n" for name in names))
        return artifacts, manifest

    def test_staging_keeps_desktop_and_updater_files_with_exact_hashes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            artifacts, manifest = self.fixture(root)
            before = manifest.read_bytes()
            output = root / "desktop"
            self.assertEqual(stage(artifacts, manifest, output, "0.2.0"), 18)
            self.assertEqual(len(list(output.iterdir())), 19)
            self.assertEqual(manifest.read_bytes(), before)
            self.assertFalse(any(p.name.endswith((".whl", ".crate", ".tgz")) for p in output.iterdir()))
            self.assertFalse((output / "rexafs-0.2.0.tar.gz").exists())
            for line in (output / "SHA256SUMS").read_text().splitlines():
                digest, name = line.split("  ", 1)
                self.assertEqual(hashlib.sha256((output / name).read_bytes()).hexdigest(), digest)
                self.assertEqual((output / name).read_bytes(), (artifacts / name).read_bytes())

    def test_missing_tampered_duplicate_or_wrong_version_cannot_be_staged(self):
        for defect in ["missing", "tampered", "duplicate", "version", "partial-installer"]:
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                artifacts, manifest = self.fixture(root)
                archive = artifacts / "rexafs-0.2.0-aarch64-apple-darwin.zip"
                if defect == "missing":
                    archive.unlink()
                elif defect == "tampered":
                    archive.write_bytes(b"changed")
                elif defect == "duplicate":
                    (artifacts / "duplicate").mkdir()
                    (artifacts / "duplicate" / archive.name).write_bytes(archive.read_bytes())
                elif defect == "partial-installer":
                    manifest.write_text("".join(line for line in manifest.read_text().splitlines(keepends=True) if not line.endswith(".dmg.json\n")))
                with self.assertRaises(ValueError):
                    stage(artifacts, manifest, root / "desktop", "0.3.0" if defect == "version" else "0.2.0")
                self.assertFalse((root / "desktop").exists())

    def test_next_release_requires_both_arm64_archives_and_windows_installer(self):
        for version in ["0.2.5", "0.2.5-rc.1", "0.2.11"]:
            with self.subTest(version=version), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                artifacts, manifest = self.fixture(root, version)
                self.assertEqual(stage(artifacts, manifest, root / "desktop", version), 26)
                for target in ("aarch64-unknown-linux-gnu", "aarch64-pc-windows-msvc"):
                    self.assertTrue(any(target in path.name for path in (root / "desktop").iterdir()))
        for suffix in ("aarch64-unknown-linux-gnu.tar.gz",
                       "aarch64-unknown-linux-gnu.tar.gz.sha256",
                       "aarch64-pc-windows-msvc.zip",
                       "aarch64-pc-windows-msvc-setup.exe",
                       "aarch64-pc-windows-msvc-setup.exe.qualification.json"):
            with self.subTest(missing=suffix), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                artifacts, manifest = self.fixture(root, "0.2.5")
                missing = "rexafs-0.2.5-" + suffix
                manifest.write_text("".join(line for line in manifest.read_text().splitlines(keepends=True)
                                            if not line.endswith("  " + missing + "\n")))
                with self.assertRaises(ValueError):
                    stage(artifacts, manifest, root / "desktop", "0.2.5")
                self.assertFalse((root / "desktop").exists())

    def test_published_0_2_4_keeps_its_four_target_manifest(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            artifacts, manifest = self.fixture(root, "0.2.4")
            self.assertEqual(stage(artifacts, manifest, root / "desktop", "0.2.4"), 18)
            self.assertFalse(any("aarch64-pc-windows" in path.name or "aarch64-unknown-linux" in path.name
                                 for path in (root / "desktop").iterdir()))

    def test_0_2_12_drops_intel_mac_without_weakening_other_targets(self):
        for version in ("0.2.12", "0.2.12-rc.1", "0.3.0"):
            with self.subTest(version=version), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                artifacts, manifest = self.fixture(root, version)
                output = root / "desktop"
                self.assertEqual(stage(artifacts, manifest, output, version), 21)
                self.assertFalse(any("x86_64-apple-darwin" in path.name for path in output.iterdir()))
                self.assertEqual(set(targets_for_version(version)), {
                    "aarch64-apple-darwin", "x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc",
                    "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"})
                original = manifest.read_text()
                for target in targets_for_version(version):
                    manifest.write_text("".join(line for line in original.splitlines(keepends=True)
                                                if f"rexafs-{version}-{target}." not in line))
                    with self.subTest(missing=target), self.assertRaises(ValueError):
                        stage(artifacts, manifest, root / "incomplete", version)
                manifest.write_text(original + f"{'0' * 64}  rexafs-{version}-x86_64-apple-darwin.zip\n")
                with self.assertRaisesRegex(ValueError, "target not supported"):
                    stage(artifacts, manifest, root / "retired", version)


if __name__ == "__main__":
    unittest.main()
