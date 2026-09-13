"""Reject inconsistent coordinated versions before release builds and publication."""

import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

script = Path(__file__).with_name("check-release-version.py")
spec = importlib.util.spec_from_file_location("release_version", script)
if spec is None or spec.loader is None:
    raise ImportError(f"Cannot load release version checker: {script}")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseVersionTests(unittest.TestCase):
    def fixture(self, root, version="0.2.5"):
        def write(path, value):
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(value, encoding="utf-8")

        write(
            "Cargo.toml",
            f'''[workspace.package]
version = "{version}"
[workspace.dependencies]
rexafs = {{ version = "{version}", path = "crates/rexafs" }}
''',
        )
        for directory, name in release.MEMBERS.items():
            write(
                f"{directory}/Cargo.toml",
                f'[package]\nname = "{name}"\nversion.workspace = true\n',
            )
        write(
            "Cargo.lock",
            "version = 4\n"
            + "".join(
                f'\n[[package]]\nname = "{name}"\nversion = "{version}"\n'
                for name in release.MEMBERS.values()
            ),
        )
        package = {"name": "rexafs", "version": version}
        write("js-rexafs/package.json", json.dumps(package))
        write(
            "js-rexafs/package-lock.json",
            json.dumps({**package, "packages": {"": package}}),
        )
        write(
            "pyproject.toml",
            """[project]
name = "rexafs"
dynamic = ["version"]
[build-system]
build-backend = "maturin"
[tool.maturin]
manifest-path = "py-rexafs/Cargo.toml"
""",
        )
        # Published documentation may lag source, and the private website is
        # versioned independently. Neither belongs to the coordinated bump.
        write("website/src/data/release.json", '{"version":"0.2.4"}')
        write("website/package.json", '{"version":"0.1.0","private":true}')

    def test_stable_and_supported_prereleases_leave_published_metadata_independent(
        self,
    ):
        for version in [
            "0.2.5",
            "1.0.0",
            "0.2.5-alpha.0",
            "0.2.5-beta.2",
            "0.2.5-rc.1",
        ]:
            with (
                self.subTest(version=version),
                tempfile.TemporaryDirectory() as temporary,
            ):
                root = Path(temporary)
                self.fixture(root, version)
                self.assertEqual(release.validate(root), version)
                self.assertEqual(release.validate(root, f"v{version}"), version)

    def test_rejects_unsupported_versions_and_wrong_source_tag(self):
        for version in [
            "v0.2.5",
            "01.2.5",
            "0.2.5+build.1",
            "0.2.5-nightly.1",
            "0.2.5-rc.01",
        ]:
            with (
                self.subTest(version=version),
                tempfile.TemporaryDirectory() as temporary,
            ):
                root = Path(temporary)
                self.fixture(root, version)
                with self.assertRaisesRegex(ValueError, "Unsupported coordinated"):
                    release.validate(root)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            for tag in ["v0.2.4", "0.2.5", "v0.2.5-publish-tools.1", ""]:
                with (
                    self.subTest(tag=tag),
                    self.assertRaisesRegex(ValueError, "release tag"),
                ):
                    release.validate(root, tag)

    def test_rejects_stale_workspace_dependency_and_member_version(self):
        cases = [
            (
                "Cargo.toml",
                'version = "0.2.5", path',
                'version = "0.2.4", path',
                "workspace.dependencies.rexafs.version",
            ),
            (
                "Cargo.toml",
                'path = "crates/rexafs"',
                'path = "other"',
                "workspace.dependencies.rexafs.path",
            ),
            *[
                (
                    f"{directory}/Cargo.toml",
                    "version.workspace = true",
                    'version = "0.2.5"',
                    "package.version",
                )
                for directory in release.MEMBERS
            ],
        ]
        for path, old, new, message in cases:
            with self.subTest(path=path), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.fixture(root)
                file = root / path
                file.write_text(file.read_text().replace(old, new), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, message):
                    release.validate(root)

    def test_rejects_each_stale_cargo_lock_member_and_nonlocal_or_missing_entries(self):
        for name in release.MEMBERS.values():
            for defect in ["version", "missing", "duplicate", "registry"]:
                with (
                    self.subTest(name=name, defect=defect),
                    tempfile.TemporaryDirectory() as temporary,
                ):
                    root = Path(temporary)
                    self.fixture(root)
                    path = root / "Cargo.lock"
                    entry = f'\n[[package]]\nname = "{name}"\nversion = "0.2.5"\n'
                    replacement = {
                        "version": entry.replace("0.2.5", "0.2.4"),
                        "missing": "",
                        "duplicate": entry + entry,
                        "registry": entry
                        + 'source = "registry+https://example.invalid/index"\n',
                    }[defect]
                    path.write_text(
                        path.read_text().replace(entry, replacement), encoding="utf-8"
                    )
                    with self.assertRaisesRegex(ValueError, f"Cargo.lock {name}"):
                        release.validate(root)

    def test_rejects_each_stale_npm_manifest_and_lockfile_version(self):
        for relative, nested in [
            ("package.json", False),
            ("package-lock.json", False),
            ("package-lock.json", True),
        ]:
            with (
                self.subTest(relative=relative, nested=nested),
                tempfile.TemporaryDirectory() as temporary,
            ):
                root = Path(temporary)
                self.fixture(root)
                path = root / "js-rexafs" / relative
                document = json.loads(path.read_text())
                package = document["packages"][""] if nested else document
                package["version"] = "0.2.4"
                path.write_text(json.dumps(document), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "js-rexafs/.*version"):
                    release.validate(root)

    def test_rejects_python_static_version_or_another_version_source(self):
        for old, new, message in [
            (
                'dynamic = ["version"]',
                "dynamic = []",
                "derive project.version dynamically",
            ),
            (
                'dynamic = ["version"]',
                'dynamic = ["version"]\nversion = "0.2.5"',
                "derive project.version dynamically",
            ),
            ('build-backend = "maturin"', 'build-backend = "other"', "build-backend"),
            (
                'manifest-path = "py-rexafs/Cargo.toml"',
                'manifest-path = "other/Cargo.toml"',
                "manifest-path",
            ),
        ]:
            with (
                self.subTest(message=message, new=new),
                tempfile.TemporaryDirectory() as temporary,
            ):
                root = Path(temporary)
                self.fixture(root)
                path = root / "pyproject.toml"
                path.write_text(path.read_text().replace(old, new), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, message):
                    release.validate(root)

    def test_cli_checks_remain_enabled_under_python_optimization(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            (root / "scripts").mkdir()
            copied = root / "scripts" / script.name
            shutil.copyfile(script, copied)
            correct = subprocess.run(
                [sys.executable, "-O", str(copied), "v0.2.5"],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(correct.returncode, 0, correct.stderr)
            self.assertEqual(correct.stdout.strip(), "0.2.5")
            wrong = subprocess.run(
                [sys.executable, "-O", str(copied), "v0.2.4"],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(wrong.returncode, 0)
            self.assertIn("release tag", wrong.stderr)
            (root / "Cargo.lock").unlink()
            missing = subprocess.run(
                [sys.executable, str(copied)],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn("Release version check failed", missing.stderr)


if __name__ == "__main__":
    unittest.main()
