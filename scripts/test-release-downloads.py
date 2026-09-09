"""Keep registry packages off the desktop download list without weakening hashes."""
import hashlib
from pathlib import Path
import tempfile
import unittest

from release_downloads import stage, TARGETS


class ReleaseDownloadsTests(unittest.TestCase):
    def fixture(self, root):
        artifacts = root / "artifacts"
        artifacts.mkdir()
        names = ["rexafs-0.2.0.crate", "rexafs-0.2.0.tgz", "rexafs-0.2.0.tar.gz", "rexafs-0.2.0-cp312-cp312-win_amd64.whl"]
        for target, extension in TARGETS.items():
            stem = f"rexafs-0.2.0-{target}"
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


if __name__ == "__main__":
    unittest.main()
