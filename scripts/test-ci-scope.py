"""Check CI scope against real Git histories and conservative path boundaries."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import ci_scope

SCRIPT = Path(__file__).with_name("ci_scope.py")


class PathScopeTests(unittest.TestCase):
    def test_exact_editorial_inventory_does_not_exempt_other_csv_files(self):
        self.assertFalse(
            ci_scope.classify_paths(["doc/documentation-audience.csv"]).full
        )
        for path in [
            "doc/neighboring.csv",
            "doc/documentation-audience.csv.bak",
            "doc/benchmarks/documentation-audience.csv",
        ]:
            with self.subTest(path=path):
                self.assertTrue(ci_scope.classify_paths([path]).full)

    def test_website_and_reviewed_prose_preserve_the_fast_path(self):
        paths = [
            "README.md",
            "CONTRIBUTING.md",
            "AGENTS.md",
            "CHANGELOG.md",
            "website/src/browser/scattering.ts",
            "website/vendor/refeff/LICENSE",
            "website/src/data/release.json",
            "website/package-lock.json",
            "doc/installing.md",
            "doc/science.rst",
            "doc/api.mdx",
            "doc/validation/figure 1.png",
            "doc/plots/fit.svg",
            "doc/plots/report.pdf",
            "doc/validation/index.html",
            "doc/validation/theme.css",
            "doc/benchmarks/report.md",
        ]
        self.assertFalse(ci_scope.classify_paths(paths).full)

    def test_package_inputs_and_unknown_files_never_qualify_as_just_prose(self):
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            ".cargo/config.toml",
            "rust-toolchain.toml",
            "crates/rexafs/README.md",
            "crates/rexafs/tests/testfiles/README.md",
            "crates/rexafs-gui/tests/fixtures/projects/manifest.json",
            "py-rexafs/README.md",
            "pyproject.toml",
            "js-rexafs/README.md",
            "vendor/sum_tree/README.rexafs.md",
            "LICENSE-MIT",
            "COPYRIGHT.md",
            "deny.toml",
            "assets/brand/README.md",
            "assets/licenses/NOTICE.txt",
            ".github/workflows/website.yml",
            "scripts/ci_scope.py",
            ".pre-commit-config.yaml",
            ".gitignore",
            ".gitattributes",
            "new-directory/README.md",
            "doc/unknown.extension",
            "doc/input.txt",
            "doc/benchmarks/raw-cases.zip",
            "doc/benchmarks/input.json",
            "doc/plots/feff_vs_larch_data/01_path_builder.csv",
            "doc/validation/refresh_gallery.py",
            "doc/validation/check.sh",
        ]:
            with self.subTest(path=path):
                self.assertTrue(
                    ci_scope.classify_paths(["website/index.astro", path]).full
                )

    def test_invalid_paths_and_empty_scope_fail_closed(self):
        for path in [
            "",
            "/website/index.md",
            "../website/index.md",
            "website/../Cargo.toml",
            "website/./index.md",
            "website//index.md",
            "website/",
            "website\\index.md",
            "C:/website/index.md",
            "website/x:y.md",
            "website/new\nfull=false",
            "website/tab\tfile.md",
            "website/\x00index.md",
            "website/\x7findex.md",
            "website/\udcffindex.md",
            "website",
            "website-copy/index.md",
            "doc",
            "docs/installing.md",
        ]:
            with self.subTest(path=repr(path)):
                self.assertTrue(ci_scope.classify_paths([path]).full)
        self.assertTrue(ci_scope.classify_paths([]).full)

    def test_reasons_cannot_inject_output_lines(self):
        scope = ci_scope.classify_paths(["website/new\nfull=false"])
        self.assertNotIn("\n", scope.reason)
        self.assertNotIn("full=false", scope.reason)

    def test_non_pr_events_and_forced_coverage_do_not_consult_the_diff(self):
        with patch.object(
            ci_scope, "changed_paths", side_effect=AssertionError("Unexpected Git call")
        ):
            for event in [
                "workflow_dispatch",
                "workflow_call",
                "push",
                "schedule",
                "pull_request_target",
                "",
                "unknown",
            ]:
                with self.subTest(event=event):
                    self.assertTrue(
                        ci_scope.select_scope(event, "a" * 40, "b" * 40).full
                    )
            self.assertTrue(
                ci_scope.select_scope(
                    "pull_request", "a" * 40, "b" * 40, force_full=True
                ).full
            )

    def test_missing_history_and_git_failures_require_full_qualification(self):
        for base, head in [(None, None), ("a" * 40, None), (None, "b" * 40)]:
            self.assertTrue(ci_scope.select_scope("pull_request", base, head).full)
        for error in [
            OSError("Git missing"),
            subprocess.CalledProcessError(128, "git"),
            ValueError("Bad output"),
        ]:
            with (
                self.subTest(error=error),
                patch.object(ci_scope, "changed_paths", side_effect=error),
            ):
                self.assertTrue(
                    ci_scope.select_scope("pull_request", "a" * 40, "b" * 40).full
                )

    def test_push_opt_in_cannot_reduce_other_events_and_force_full_wins(self):
        with patch.object(
            ci_scope, "changed_paths", side_effect=AssertionError("Unexpected Git call")
        ):
            for event in [
                "workflow_dispatch",
                "workflow_call",
                "schedule",
                "unknown",
                "pull_request_target",
            ]:
                with self.subTest(event=event):
                    self.assertTrue(
                        ci_scope.select_scope(
                            event, "a" * 40, "b" * 40, scope_push=True
                        ).full
                    )
            self.assertTrue(
                ci_scope.select_scope(
                    "push", "a" * 40, "b" * 40, scope_push=True, force_full=True
                ).full
            )


class GitScopeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.env = {
            **os.environ,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_AUTHOR_NAME": "CI scope test",
            "GIT_AUTHOR_EMAIL": "ci-scope@example.invalid",
            "GIT_COMMITTER_NAME": "CI scope test",
            "GIT_COMMITTER_EMAIL": "ci-scope@example.invalid",
        }
        self.git("init", "--quiet", "--initial-branch=main")
        self.write("README.md", "Original prose\n")
        self.write("crates/rexafs/src/lib.rs", "pub fn original() {}\n")
        self.base = self.commit("Base")

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args],
            cwd=self.root,
            env=self.env,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()

    def write(self, path: str, contents: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(contents, encoding="utf-8")

    def commit(self, message: str) -> str:
        self.git("add", "--all")
        self.git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", message)
        return self.git("rev-parse", "HEAD")

    def scope(self, head: str, base: str | None = None) -> ci_scope.Scope:
        return ci_scope.select_scope(
            "pull_request", base or self.base, head, repository=self.root
        )

    def test_rename_from_native_into_website_checks_the_removed_native_path(self):
        destination = self.root / "website/example.md"
        destination.parent.mkdir()
        (self.root / "crates/rexafs/src/lib.rs").rename(destination)
        head = self.commit("Move native source into website")
        self.assertEqual(
            set(ci_scope.changed_paths(self.base, head, self.root)),
            {"crates/rexafs/src/lib.rs", "website/example.md"},
        )
        self.assertTrue(self.scope(head).full)

    def test_native_deletion_requires_full_but_prose_deletion_does_not(self):
        (self.root / "README.md").unlink()
        prose_head = self.commit("Remove prose")
        self.assertFalse(self.scope(prose_head).full)
        (self.root / "crates/rexafs/src/lib.rs").unlink()
        native_head = self.commit("Remove native source")
        self.assertTrue(self.scope(native_head).full)

    def test_complete_commit_ids_ignore_checkout_branch_and_working_tree(self):
        self.write("website/guide with spaces.md", "Updated guide\n")
        docs_head = self.commit("Website guide")
        self.write("crates/rexafs/src/lib.rs", "pub fn changed() {}\n")
        native_head = self.commit("Native change after reviewed docs")
        self.write("README.md", "Uncommitted prose\n")
        self.assertFalse(self.scope(docs_head).full)
        self.assertTrue(self.scope(native_head).full)
        for mutable_or_short in [
            "HEAD",
            "main",
            docs_head[:12],
            f"{docs_head}^",
            "--all",
        ]:
            with self.subTest(ref=mutable_or_short):
                self.assertTrue(self.scope(mutable_or_short).full)
                self.assertTrue(self.scope(docs_head, base=mutable_or_short).full)

    def test_merge_base_excludes_unrelated_target_branch_changes(self):
        self.git("switch", "--quiet", "-c", "docs")
        self.write("website/guide.md", "Guide\n")
        docs_head = self.commit("PR docs")
        self.git("switch", "--quiet", "main")
        self.write("crates/rexafs/src/lib.rs", "pub fn target_branch_change() {}\n")
        advanced_base = self.commit("Unrelated target branch change")
        self.assertFalse(self.scope(docs_head, base=advanced_base).full)
        self.assertEqual(
            ci_scope.changed_paths(advanced_base, docs_head, self.root),
            ["website/guide.md"],
        )

    def test_push_scope_is_opt_in_and_checks_native_changes(self):
        self.write("website/guide.md", "Guide\n")
        docs_head = self.commit("Docs")
        self.assertTrue(
            ci_scope.select_scope(
                "push", self.base, docs_head, repository=self.root
            ).full
        )
        self.assertFalse(
            ci_scope.select_scope(
                "push", self.base, docs_head, scope_push=True, repository=self.root
            ).full
        )
        self.write("crates/rexafs/src/lib.rs", "pub fn changed() {}\n")
        native_head = self.commit("Native")
        self.assertTrue(
            ci_scope.select_scope(
                "push", docs_head, native_head, scope_push=True, repository=self.root
            ).full
        )
        self.assertTrue(
            ci_scope.select_scope(
                "push", "0" * 40, docs_head, scope_push=True, repository=self.root
            ).full
        )

    def test_push_compares_trees_to_detect_native_removals_from_a_divergent_tip(self):
        self.git("switch", "--quiet", "-c", "docs")
        self.write("website/guide.md", "Guide\n")
        docs_head = self.commit("Docs")
        self.git("switch", "--quiet", "main")
        self.write("crates/rexafs/src/extra.rs", "pub fn extra() {}\n")
        previous_tip = self.commit("Native source on previous branch tip")
        self.assertFalse(self.scope(docs_head, base=previous_tip).full)
        self.assertTrue(
            ci_scope.select_scope(
                "push", previous_tip, docs_head, scope_push=True, repository=self.root
            ).full
        )
        self.assertEqual(
            set(
                ci_scope.changed_paths(
                    previous_tip, docs_head, self.root, merge_base=False
                )
            ),
            {"crates/rexafs/src/extra.rs", "website/guide.md"},
        )

    def test_empty_unknown_and_non_commit_objects_require_full_qualification(self):
        self.assertTrue(self.scope(self.base).full)
        self.assertTrue(self.scope("0" * 40).full)
        blob = self.git("rev-parse", "HEAD:README.md")
        self.assertTrue(self.scope(blob).full)
        self.git(
            "-c", "tag.gpgsign=false", "tag", "-a", "test-tag", "-m", "Annotated tag"
        )
        self.assertTrue(self.scope(self.git("rev-parse", "refs/tags/test-tag")).full)

    def test_shallow_checkout_cannot_skip_checks_without_the_base_commit(self):
        self.write("website/guide.md", "Guide\n")
        head = self.commit("Docs")
        with tempfile.TemporaryDirectory() as temporary:
            checkout = Path(temporary) / "shallow"
            subprocess.run(
                [
                    "git",
                    "clone",
                    "--quiet",
                    "--depth=1",
                    self.root.as_uri(),
                    str(checkout),
                ],
                env=self.env,
                capture_output=True,
                check=True,
            )
            self.assertTrue(
                ci_scope.select_scope(
                    "pull_request", self.base, head, repository=checkout
                ).full
            )

    def test_nul_diff_does_not_split_whitespace_or_accept_output_injection(self):
        self.write("website/file with spaces.md", "Guide\n")
        spaced = self.commit("Filename with spaces")
        self.assertFalse(self.scope(spaced).full)
        self.write("website/new\nfull=false", "Unusual filename\n")
        unusual = self.commit("Filename with newline")
        self.assertTrue(self.scope(unusual).full)
        self.assertIn(
            "website/new\nfull=false",
            ci_scope.changed_paths(self.base, unusual, self.root),
        )

    def test_cli_outputs_append_and_manual_events_cannot_reduce_coverage(self):
        self.write("website/guide.md", "Guide\n")
        head = self.commit("Docs")
        output = self.root / "github-output"
        output.write_text("previous=value\n", encoding="utf-8")
        for event, extra, expected in [
            ("pull_request", [], "false"),
            ("workflow_dispatch", [], "true"),
            ("pull_request", ["--force-full"], "true"),
            ("push", [], "true"),
            ("push", ["--scope-push"], "false"),
            ("push", ["--scope-push", "--force-full"], "true"),
            ("workflow_dispatch", ["--scope-push"], "true"),
            ("workflow_call", ["--scope-push"], "true"),
        ]:
            with self.subTest(event=event, extra=extra):
                result = subprocess.run(
                    [
                        sys.executable,
                        "-O",
                        str(SCRIPT),
                        "--event",
                        event,
                        "--base",
                        self.base,
                        "--head",
                        head,
                        "--github-output",
                        str(output),
                        *extra,
                    ],
                    cwd=self.root,
                    env=self.env,
                    capture_output=True,
                    text=True,
                    check=True,
                )
                self.assertTrue(
                    result.stdout.startswith(f"full={expected}\nreason="), result.stdout
                )
                self.assertEqual(len(result.stdout.splitlines()), 2)
                self.assertTrue(
                    output.read_text(encoding="utf-8").endswith(result.stdout)
                )
        self.assertTrue(
            output.read_text(encoding="utf-8").startswith("previous=value\n")
        )

    def test_cli_output_write_failure_is_not_a_successful_skip(self):
        self.write("website/guide.md", "Guide\n")
        head = self.commit("Docs")
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--event",
                "pull_request",
                "--base",
                self.base,
                "--head",
                head,
                "--github-output",
                str(self.root / "missing-directory" / "output"),
            ],
            cwd=self.root,
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("full=false", result.stdout)


if __name__ == "__main__":
    unittest.main()
