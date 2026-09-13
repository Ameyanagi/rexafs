"""Regression tests for shared-wheel identity, platform inventory and installed bytes."""

import hashlib
import importlib.util
import io
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import python_wheels as wheels


def make_wheel(root, runner="macos-15", version="0.2.6", edits=None):
    """Create a small native-wheel fixture; it is never imported or distributed."""
    tags = sorted(wheels.PLATFORMS[runner])
    filename = f"rexafs-{wheels.python_version(version)}-cp310-abi3-{'.'.join(tags)}.whl"
    dist = f"rexafs-{wheels.python_version(version)}.dist-info"
    files = {name: b"fixture" for name in (
        "rexafs/__init__.py", "rexafs/__init__.pyi", "rexafs/io.py", "rexafs/io.pyi", "rexafs/py.typed")}
    extension = "_core.pyd" if runner == "windows-2025" else "_core.abi3.so"
    files[f"rexafs/{extension}"] = b"native fixture, not executable"
    files[f"{dist}/METADATA"] = (
        f"Name: rexafs\nVersion: {wheels.python_version(version)}\n"
        "Requires-Python: >=3.10, <3.15\nRequires-Dist: numpy>=1.23\n\n"
    ).encode()
    files[f"{dist}/WHEEL"] = (
        "Wheel-Version: 1.0\nRoot-Is-Purelib: false\n"
        + "".join(f"Tag: cp310-abi3-{tag}\n" for tag in tags) + "\n"
    ).encode()
    for name, value in (edits or {}).items():
        if value is None:
            files.pop(name)
        else:
            files[name] = value
    path = root / filename
    with zipfile.ZipFile(path, "w") as archive:
        for name, data in files.items():
            archive.writestr(name, data)
    return path


class WheelTests(unittest.TestCase):
    def test_source_archive_must_retain_the_stable_abi_feature(self):
        spec = importlib.util.spec_from_file_location("sdist", Path(__file__).with_name("check-python-sdist.py"))
        sdist = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(sdist)
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "rexafs-0.2.6.tar.gz"
            for features, valid in (("'extension-module', 'abi3-py310'", True),
                                    ("'extension-module'", False),
                                    ("'abi3-py311'", False)):
                with tarfile.open(archive, "w:gz") as tar:
                    files = {"PKG-INFO": b"Name: rexafs\nVersion: 0.2.6\n\n",
                             "py-rexafs/Cargo.toml": f"[dependencies]\npyo3 = {{features = [{features}]}}\n".encode()}
                    for name, data in files.items():
                        member = tarfile.TarInfo(f"rexafs-0.2.6/{name}")
                        member.size = len(data)
                        tar.addfile(member, io.BytesIO(data))
                if valid:
                    sdist.check(archive, require_abi3=True)
                else:
                    with self.subTest(features=features), self.assertRaises(ValueError):
                        sdist.check(archive, require_abi3=True)

    def test_all_platforms_and_prerelease_metadata(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for version in ("0.2.6", "0.2.6-rc.1"):
                names = []
                for runner in wheels.PLATFORMS:
                    path = make_wheel(root, runner, version)
                    self.assertEqual(wheels.check_wheel(path, version, runner), hashlib.sha256(path.read_bytes()).hexdigest())
                    names.append(path.name)
                wheels.inventory(names, version)

    def test_incomplete_duplicate_or_mixed_inventory_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            names = [make_wheel(Path(temporary), runner).name for runner in wheels.PLATFORMS]
            for invalid in ([], names[:-1], names + names[:1], names[:-1] + names[:1],
                            [name.replace("cp310-abi3", "cp312-cp312") for name in names]):
                with self.subTest(names=invalid), self.assertRaises(ValueError):
                    wheels.inventory(invalid, "0.2.6")

    def test_filename_cannot_widen_python_abi_or_platform_support(self):
        name = "rexafs-0.2.6-cp310-abi3-macosx_11_0_arm64.whl"
        replacements = (("0.2.6", "0.2.7"), ("cp310", "cp311"), ("abi3", "abi3t"),
                        ("arm64", "x86_64"), ("macosx_11_0", "macosx_14_0"),
                        (".whl", ".macosx_11_0_arm64.whl"))
        for original, replacement in replacements:
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                wheels.identity(name.replace(original, replacement), "0.2.6")

    def test_metadata_native_extension_and_editor_files_are_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            invalid = (
                {"rexafs-0.2.6.dist-info/METADATA": b"Name: other\nVersion: 0.2.6\n\n"},
                {"rexafs-0.2.6.dist-info/WHEEL": b"Root-Is-Purelib: false\nTag: cp312-cp312-macosx_11_0_arm64\n\n"},
                {"rexafs/_core.abi3.so": None},
                {"rexafs/_core.cpython-312-darwin.so": b"extra native binary"},
                {"rexafs/__init__.pyi": None},
                {"rexafs/py.typed": None},
            )
            for edits in invalid:
                with self.subTest(edits=edits), self.assertRaises(ValueError):
                    wheels.check_wheel(make_wheel(root, edits=edits), "0.2.6")
            with self.assertRaises(ValueError):
                wheels.check_wheel(make_wheel(root), "0.2.6", "windows-2025")

    def test_installation_must_match_wheel_bytes_and_selected_environment(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            wheel = make_wheel(root)
            site = root / "site-packages"
            package = site / "rexafs"
            package.mkdir(parents=True)
            with zipfile.ZipFile(wheel) as archive:
                for name in archive.namelist():
                    if name.startswith("rexafs/"):
                        (site / name).write_bytes(archive.read(name))
            modules = {
                "rexafs": SimpleNamespace(__file__=str(package / "__init__.py"), __version__="0.2.6"),
                "rexafs._core": SimpleNamespace(__file__=str(package / "_core.abi3.so")),
                "numpy": SimpleNamespace(__version__="1.26.0"),
            }
            with (patch.object(wheels.importlib, "import_module", side_effect=modules.__getitem__),
                  patch.object(wheels.importlib.metadata, "distribution", return_value=SimpleNamespace(version="0.2.6")) as distribution,
                  patch.object(wheels.sysconfig, "get_path", return_value=str(site)),
                  patch.object(wheels.sysconfig, "get_config_var", return_value=0)):
                evidence = wheels.check_installed(wheel, "0.2.6", "1.26.0")
                self.assertEqual(evidence["numpy"], "1.26.0")
                prerelease = make_wheel(root, version="0.2.6-rc.1")
                modules["rexafs"].__version__ = "0.2.6-rc.1"
                distribution.return_value.version = "0.2.6rc1"
                self.assertEqual(wheels.check_installed(prerelease, "0.2.6-rc.1")["rexafs"], "0.2.6-rc.1")
                modules["rexafs"].__version__ = "0.2.6"
                distribution.return_value.version = "0.2.6"
                with self.assertRaises(ValueError):
                    wheels.check_installed(wheel, "0.2.6", "2.0.0")
                for name in ("_core.abi3.so", "__init__.pyi", "__init__.py"):
                    path = package / name
                    original = path.read_bytes()
                    path.write_bytes(b"different installed bytes")
                    with self.subTest(name=name), self.assertRaises(ValueError):
                        wheels.check_installed(wheel, "0.2.6")
                    path.write_bytes(original)
                with patch.object(wheels.sysconfig, "get_path", return_value=str(root / "other")), self.assertRaises(ValueError):
                    wheels.check_installed(wheel, "0.2.6")
                with patch.object(wheels.sysconfig, "get_config_var", return_value=1), self.assertRaises(ValueError):
                    wheels.check_installed(wheel, "0.2.6")

    def test_publication_profile_follows_source_features(self):
        spec = importlib.util.spec_from_file_location("source", Path(__file__).with_name("check-release-source.py"))
        source = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(source)
        for features, expected in ((["extension-module"], "per-interpreter"),
                                   (["extension-module", "abi3-py310"], "abi3-py310")):
            self.assertEqual(source.python_abi({"dependencies": {"pyo3": {"features": features}}}), expected)
        for features in (["abi3"], ["abi3-py311"], ["abi3-py310", "abi3-py311"], ["abi3t-py315"]):
            with self.subTest(features=features), self.assertRaises(ValueError):
                source.python_abi({"dependencies": {"pyo3": {"features": features}}})

    def test_publication_rejects_missing_mixed_or_modified_abi3_wheels(self):
        spec = importlib.util.spec_from_file_location("registry", Path(__file__).with_name("check-registry-artifacts.py"))
        registry = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(registry)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifacts = root / "artifacts"
            artifacts.mkdir()
            paths = [make_wheel(artifacts, runner) for runner in wheels.PLATFORMS]
            sdist = artifacts / "rexafs-0.2.6.tar.gz"
            sdist.write_bytes(b"source archive fixture")
            manifest = root / "SHA256SUMS"
            manifest.write_text("".join(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n" for path in [*paths, sdist]))
            self.assertEqual(registry.verify("pypi", artifacts, manifest, "0.2.6", "abi3-py310"), 5)
            with self.assertRaises(ValueError):
                registry.verify("pypi", artifacts, manifest, "0.2.6", "per-interpreter")
            original = manifest.read_text()
            manifest.write_text("\n".join(original.splitlines()[1:]) + "\n")
            with self.assertRaises(ValueError):
                registry.verify("pypi", artifacts, manifest, "0.2.6", "abi3-py310")
            manifest.write_text(original)
            paths[0].write_bytes(paths[0].read_bytes() + b"modified bytes")
            with self.assertRaises(ValueError):
                registry.verify("pypi", artifacts, manifest, "0.2.6", "abi3-py310")


if __name__ == "__main__":
    unittest.main()
