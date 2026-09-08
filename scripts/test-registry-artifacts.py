"""Registry publishing rejects incomplete or modified packages without desktop downloads."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("registry", Path(__file__).with_name("check-registry-artifacts.py"))
registry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(registry)


class RegistryArtifactTests(unittest.TestCase):
    def payloads(self, version="0.2.0", python_version="0.2.0"):
        return {
            f"rexafs-{version}.crate": b"original Rust crate",
            f"rexafs-{version}.tgz": b"original npm tarball",
            f"rexafs-{python_version}-cp310-cp310-manylinux_2_34_x86_64.whl": b"original Linux wheel",
            f"rexafs-{python_version}-cp314-cp314-macosx_11_0_arm64.whl": b"original Mac wheel",
            f"rexafs-{python_version}.tar.gz": b"original source archive",
            f"rexafs-{version}-x86_64-unknown-linux-gnu.tar.gz": b"unavailable desktop archive",
            f"rexafs-{version}-aarch64-apple-darwin.zip": b"unavailable Mac archive",
        }

    def manifest(self, root, payloads):
        path = root / "SHA256SUMS"
        path.write_text("".join(f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in payloads.items()))
        return path

    def copy(self, root, payloads, names):
        root.mkdir()
        for name in names:
            (root / name).write_bytes(payloads[name])

    def test_each_registry_succeeds_without_unrelated_desktop_archives(self):
        for version, python_version in [("0.2.0", "0.2.0"), ("0.2.0-rc.1", "0.2.0rc1")]:
            with self.subTest(version=version), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                payloads = self.payloads(version, python_version)
                manifest = self.manifest(root, payloads)
                for channel, suffixes in [("crates-io", (".crate",)), ("npm", (".tgz",)),
                                          ("pypi", (".whl", f"{python_version}.tar.gz"))]:
                    names = {name for name in payloads if name.endswith(suffixes)}
                    destination = root / channel
                    self.copy(destination, payloads, names)
                    self.assertEqual(registry.verify(channel, destination, manifest, version), len(names))

    def test_python_requires_every_manifest_wheel_and_the_source_archive(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payloads = self.payloads()
            manifest = self.manifest(root, payloads)
            names = {name for name in payloads if name.endswith((".whl", "0.2.0.tar.gz"))}
            destination = root / "pypi"
            self.copy(destination, payloads, names)
            for name in names:
                with self.subTest(missing=name):
                    (destination / name).unlink()
                    with self.assertRaises(ValueError):
                        registry.verify("pypi", destination, manifest, "0.2.0")
                    (destination / name).write_bytes(payloads[name])

    def test_tampered_unexpected_duplicate_and_wrong_version_packages_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payloads = self.payloads()
            manifest = self.manifest(root, payloads)
            destination = root / "npm"
            name = "rexafs-0.2.0.tgz"
            self.copy(destination, payloads, {name})
            (destination / name).write_bytes(b"modified")
            with self.assertRaises(ValueError):
                registry.verify("npm", destination, manifest, "0.2.0")
            (destination / name).write_bytes(payloads[name])
            (destination / "unexpected.whl").write_bytes(b"unexpected")
            with self.assertRaises(ValueError):
                registry.verify("npm", destination, manifest, "0.2.0")
            (destination / "unexpected.whl").unlink()
            duplicate = destination / "nested"
            duplicate.mkdir()
            (duplicate / name).write_bytes(payloads[name])
            with self.assertRaises(ValueError):
                registry.verify("npm", destination, manifest, "0.2.0")
            (duplicate / name).unlink()
            for channel in ("npm", "pypi"):
                with self.subTest(channel=channel), self.assertRaises(ValueError):
                    registry.verify(channel, destination, manifest, "0.2.1")

    def test_manifest_cannot_hide_duplicate_entries_or_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = self.manifest(root, self.payloads())
            original = manifest.read_text()
            for invalid in [original + original.splitlines()[0] + "\n",
                            "0" * 64 + "  nested/package.whl\n",
                            "invalid  package.whl\n"]:
                manifest.write_text(invalid)
                with self.assertRaises(ValueError):
                    registry.read_manifest(manifest)


if __name__ == "__main__":
    unittest.main()
