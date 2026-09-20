"""Channel isolation, source/signing identity and nightly publication regressions."""
import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from zipfile import ZipFile
from desktop_channels import app_name, identity


def module(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + ".py"))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


nightly = module("nightly-release")
macos = module("sign-macos-release")
TAG = "nightly-20260907-123"


class DesktopChannels(unittest.TestCase):
    def test_reruns_keep_the_original_date_and_source_identity(self):
        run = dict(id=123, head_sha="abc", head_branch="dev", event="workflow_dispatch", created_at="2026-09-07T23:59:59Z")
        expected = dict(version="0.1.2", commit="abc", tag=TAG, built_at="2026-09-07T23:59:59Z")
        self.assertEqual(nightly.build_plan(run, "0.1.2", "abc", "123"), expected)
        self.assertEqual(nightly.build_plan({**run, "event": "push"}, "0.1.2", "abc", "123"), expected)
        for key, value in [("head_sha", "other"), ("head_branch", "main"),
                           ("head_branch", "feature/example"), ("event", "pull_request"),
                           ("event", "schedule"), ("id", 124)]:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                nightly.build_plan({**run, key: value}, "0.1.2", "abc", "123")

    def test_build_identity_and_app_names_keep_channels_separate(self):
        self.assertEqual(identity("0.1.2", {})["release_tag"], "v0.1.2")
        self.assertEqual(app_name("stable"), "rexafs.app")
        self.assertEqual(app_name("nightly"), "rexafs Nightly.app")
        env = dict(REXAFS_BUILD_CHANNEL="nightly", REXAFS_BUILD_TAG=TAG, REXAFS_BUILD_UTC="2026-09-07T00:00:00Z")
        self.assertEqual(identity("0.1.2", env)["channel"], "nightly")
        for invalid in [dict(REXAFS_BUILD_CHANNEL="unexpected"), dict(REXAFS_BUILD_CHANNEL="nightly"),
                        {**env, "REXAFS_BUILD_TAG": "v0.1.2"}, {**env, "REXAFS_BUILD_CHANNEL": "stable"}]:
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                identity("0.1.2", invalid)

    def test_only_dev_push_or_manual_run_can_publish(self):
        env = dict(GITHUB_REF="refs/heads/dev", GITHUB_REPOSITORY="Ameyanagi/rexafs", GITHUB_EVENT_NAME="workflow_dispatch")
        nightly.require_dev(env)
        nightly.require_dev({**env, "GITHUB_EVENT_NAME": "push"})
        for key, value in [("GITHUB_REF", "refs/heads/main"), ("GITHUB_REF", "refs/heads/feature/example"),
                           ("GITHUB_REF", "refs/pull/12/merge"), ("GITHUB_REPOSITORY", "someone/rexafs"),
                           ("GITHUB_EVENT_NAME", "pull_request"), ("GITHUB_EVENT_NAME", "schedule"),
                           ("GITHUB_REF", "refs/tags/v0.1.2")]:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                nightly.require_dev({**env, key: value})

    def archive(self, root, target, overrides=None, signed=True):
        file = root / f"rexafs-0.1.2-{target}.zip"
        metadata = dict(version="0.1.2", commit="abc", target=target, github_run_id="123", signing_run_id="123",
                        signing_tools_commit="abc", channel="nightly", release_tag=TAG, dirty=False,
                        signed=signed, notarized=signed, notarization_id="test-only", apple_team_id="TEST")
        metadata.update(overrides or {})
        with ZipFile(file, "w") as zip:
            zip.writestr(file.stem + "/build.json", json.dumps(metadata))
            zip.writestr(file.stem + "/rexafs Nightly.app/Contents/MacOS/rexafs", b"test-only")
        digest = nightly.sha256(file)
        Path(str(file) + ".sha256").write_text(f"{digest}  {file.name}\n")
        image = file.with_suffix(".dmg")
        image.write_bytes(b"test-only image")
        evidence = dict(schema_version=1, build=metadata, signed=True, notarized=True, stapled=True,
                        gatekeeper_accepted=True, installation_verified=True, apple_team_id="TEST",
                        notarization_id="test-only", archive_sha256=digest, sha256=nightly.sha256(image),
                        executable_sha256=hashlib.sha256(b"test-only").hexdigest())
        Path(str(image) + ".json").write_text(json.dumps(evidence))
        Path(str(image) + ".sha256").write_text(f"{nightly.sha256(image)}  {image.name}\n")
        return file

    def test_publication_requires_signed_apple_silicon_and_matching_run(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            files = [self.archive(root, target) for target in sorted(nightly.TARGETS)]
            self.assertEqual(len(nightly.qualify(files, "0.1.2", "abc", "123", TAG)), 5)
            with self.assertRaises(ValueError):
                nightly.qualify([], "0.1.2", "abc", "123", TAG)
            for invalid in (files * 2, files + [self.archive(root, "x86_64-apple-darwin")]):
                with self.assertRaises(ValueError):
                    nightly.qualify(invalid, "0.1.2", "abc", "123", TAG)
            for overrides in [dict(signed=False), dict(notarized=False), dict(dirty=True), dict(commit="wrong"),
                              dict(signing_tools_commit="wrong"), dict(release_tag="nightly-20260906-123"),
                              dict(channel="stable"), dict(github_run_id="122"), dict(notarization_id="")]:
                files[0] = self.archive(root, sorted(nightly.TARGETS)[0], overrides)
                with self.subTest(overrides=overrides), self.assertRaises(ValueError):
                    nightly.qualify(files, "0.1.2", "abc", "123", TAG)

    def test_signing_does_not_accept_nightly_as_stable_or_another_tag(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = "aarch64-apple-darwin"
            file = self.archive(root, target, signed=False)
            args = [file, Path(str(file) + ".sha256"), "0.1.2", "abc", "123", target]
            macos.check_source(*args, channel="nightly", release_tag=TAG)
            with self.assertRaises(ValueError):
                macos.check_source(*args)
            with self.assertRaises(ValueError):
                macos.check_source(*args, channel="nightly", release_tag="nightly-20260906-123")
            file.write_bytes(file.read_bytes() + b"tampered")
            with self.assertRaises(ValueError):
                macos.check_source(*args, channel="nightly", release_tag=TAG)

    def test_missing_or_unqualified_installer_cannot_start_publication(self):
        for defect in ["missing", "unsigned", "extra"]:
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                files = [self.archive(root, target) for target in sorted(nightly.TARGETS)]
                image = files[0].with_suffix(".dmg")
                if defect == "missing":
                    image.unlink()
                elif defect == "unsigned":
                    evidence_file = Path(str(image) + ".json")
                    evidence = json.loads(evidence_file.read_text())
                    evidence["signed"] = False
                    evidence_file.write_text(json.dumps(evidence))
                else:
                    (root / "unexpected.dmg").write_bytes(b"test-only")
                with patch.object(nightly.subprocess, "run") as run:
                    with self.assertRaises((ValueError, FileNotFoundError)):
                        nightly.publish(root, "0.1.2", "abc", "123", TAG)
                    run.assert_not_called()

    def test_draft_lookup_uses_its_database_id_and_checks_identity(self):
        resolved = dict(databaseId=17, isDraft=True, tagName=TAG)
        remote = dict(id=17, draft=True, tag_name=TAG, assets=[])
        def lookup(command, **kwargs):
            if command[:3] == ["gh", "release", "view"]:
                return json.dumps(resolved)
            self.assertEqual(command, ["gh", "api", f"repos/{nightly.REPOSITORY}/releases/17"])
            return json.dumps(remote)
        with patch.object(nightly.subprocess, "check_output", side_effect=lookup):
            self.assertEqual(nightly.get_draft_release(TAG), remote)
            for invalid in [dict(isDraft=False), dict(tagName="another-tag"), dict(databaseId=0), dict(databaseId=True)]:
                original = resolved.copy()
                resolved.update(invalid)
                with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                    nightly.get_draft_release(TAG)
                resolved.clear()
                resolved.update(original)
            for invalid in [dict(id=18), dict(draft=False), dict(tag_name="another-tag")]:
                original = remote.copy()
                remote.update(invalid)
                with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                    nightly.get_draft_release(TAG)
                remote.clear()
                remote.update(original)

    def test_draft_publication_requires_the_exact_uploaded_asset_set(self):
        for mismatch in [None, "digest", "missing", "extra"]:
            with self.subTest(mismatch=mismatch), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                for target in sorted(nightly.TARGETS):
                    self.archive(root, target)
                calls = []
                def run(command, **kwargs):
                    calls.append(command)
                    if command[:2] == ["gh", "api"]:
                        stdout = json.dumps(dict(object=dict(type="commit", sha="abc")))
                    elif command[:3] == ["gh", "release", "view"]:
                        stdout = json.dumps(dict(isDraft=True))
                    else:
                        stdout = ""
                    return nightly.subprocess.CompletedProcess(command, 0, stdout=stdout)
                def lookup(command, **kwargs):
                    if command[:3] == ["gh", "release", "view"]:
                        return json.dumps(dict(databaseId=17, isDraft=True, tagName=TAG))
                    self.assertEqual(command, ["gh", "api", f"repos/{nightly.REPOSITORY}/releases/17"])
                    assets = [dict(name=p.name, digest="sha256:" + nightly.sha256(p))
                              for p in root.iterdir() if p.name != "nightly-notes.md"]
                    if mismatch == "digest":
                        assets[0]["digest"] = "sha256:wrong"
                    elif mismatch == "missing":
                        assets.pop()
                    elif mismatch == "extra":
                        assets.append(dict(name="unexpected.zip", digest="sha256:unexpected"))
                    return json.dumps(dict(id=17, draft=True, tag_name=TAG, assets=assets))
                with patch.object(nightly.subprocess, "run", side_effect=run), \
                        patch.object(nightly.subprocess, "check_output", side_effect=lookup):
                    if mismatch:
                        with self.assertRaisesRegex(ValueError, "Uploaded release checksums"):
                            nightly.publish(root, "0.1.2", "abc", "123", TAG)
                    else:
                        nightly.publish(root, "0.1.2", "abc", "123", TAG)
                promotions = [c for c in calls if c[:3] == ["gh", "release", "edit"]]
                self.assertEqual(len(promotions), 0 if mismatch else 1)


if __name__ == "__main__":
    unittest.main()
