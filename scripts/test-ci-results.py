"""Protect aggregate CI checks from failed selectors and incomplete job results."""

import copy
import importlib.util
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

script = Path(__file__).with_name("check-ci-results.py")
spec = importlib.util.spec_from_file_location("ci_results", script)
if spec is None or spec.loader is None:
    raise ImportError(f"Cannot load CI result checker: {script}")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class CiResultTests(unittest.TestCase):
    jobs = ("core", "python", "desktop")

    def fixture(self, full="true"):
        return {
            "scope": {"result": "success", "outputs": {"full": full}},
            **{
                name: {
                    "result": "success" if full == "true" else "skipped",
                    "outputs": {},
                }
                for name in self.jobs
            },
        }

    def cli(self, raw, *arguments, optimized=False):
        environment = os.environ.copy()
        environment.pop("CI_NEEDS_JSON", None)
        if raw is not None:
            environment["CI_NEEDS_JSON"] = raw
        return subprocess.run(
            [
                sys.executable,
                *(["-O"] if optimized else []),
                str(script),
                "--selector",
                "scope",
                *arguments,
            ],
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_complete_full_and_documentation_only_checks_preserve_input(self):
        for full in ["true", "false"]:
            with self.subTest(full=full):
                needs = self.fixture(full)
                original = copy.deepcopy(needs)
                self.assertEqual(ci.validate(needs, "scope", self.jobs), full == "true")
                self.assertEqual(needs, original)

    def test_failed_skipped_cancelled_or_missing_selector_cannot_pass(self):
        for state in ["failure", "cancelled", "skipped", "pending", "", None]:
            with self.subTest(state=state):
                needs = self.fixture("false")
                needs["scope"]["result"] = state
                with self.assertRaisesRegex(ValueError, "Selector.*success"):
                    ci.validate(needs, "scope", self.jobs)
        for selector in [{"outputs": {"full": "false"}}, None, "success"]:
            with self.subTest(selector=selector):
                needs = self.fixture("false")
                needs["scope"] = selector
                with self.assertRaisesRegex(ValueError, "Selector.*success"):
                    ci.validate(needs, "scope", self.jobs)

    def test_selector_requires_exact_string_output(self):
        for output in [
            True,
            False,
            None,
            "TRUE",
            "False",
            " true",
            "false\n",
            "",
            0,
            1,
        ]:
            with self.subTest(output=output):
                needs = self.fixture()
                needs["scope"]["outputs"]["full"] = output
                with self.assertRaisesRegex(ValueError, "outputs.full"):
                    ci.validate(needs, "scope", self.jobs)
        for outputs in [None, {}, {"different": "true"}, "true", ["true"]]:
            with self.subTest(outputs=outputs):
                needs = self.fixture()
                needs["scope"]["outputs"] = outputs
                with self.assertRaisesRegex(ValueError, "outputs.full"):
                    ci.validate(needs, "scope", self.jobs)

    def test_full_check_rejects_every_unsuccessful_collapsed_job_result(self):
        for name in self.jobs:
            for state in [
                "failure",
                "cancelled",
                "skipped",
                "neutral",
                "pending",
                "",
                None,
            ]:
                with self.subTest(job=name, state=state):
                    needs = self.fixture()
                    needs[name]["result"] = state
                    with self.assertRaisesRegex(
                        ValueError, f"{name}: expected 'success'"
                    ):
                        ci.validate(needs, "scope", self.jobs)

    def test_documentation_only_check_requires_intentional_skips(self):
        for name in self.jobs:
            for state in [
                "success",
                "failure",
                "cancelled",
                "neutral",
                "pending",
                "",
                None,
            ]:
                with self.subTest(job=name, state=state):
                    needs = self.fixture("false")
                    needs[name]["result"] = state
                    with self.assertRaisesRegex(
                        ValueError, f"{name}: expected 'skipped'"
                    ):
                        ci.validate(needs, "scope", self.jobs)

    def test_dependency_inventory_cannot_omit_or_ignore_jobs(self):
        for missing in ["scope", *self.jobs]:
            with self.subTest(missing=missing):
                needs = self.fixture()
                del needs[missing]
                with self.assertRaisesRegex(ValueError, "inventory.*missing"):
                    ci.validate(needs, "scope", self.jobs)
        for state in ["success", "failure", "skipped"]:
            with self.subTest(unexpected_result=state):
                needs = self.fixture()
                needs["unlisted_job"] = {"result": state}
                with self.assertRaisesRegex(ValueError, "unexpected=.*unlisted_job"):
                    ci.validate(needs, "scope", self.jobs)

    def test_malformed_results_and_job_identifiers_fail(self):
        for needs in [None, [], "success", False]:
            with (
                self.subTest(needs=needs),
                self.assertRaisesRegex(TypeError, "JSON object"),
            ):
                ci.validate(needs, "scope", self.jobs)
        for job in [{}, None, "success", {"result": ["success"]}]:
            with self.subTest(job=job):
                needs = self.fixture()
                needs["core"] = job
                with self.assertRaisesRegex(ValueError, "core: expected 'success'"):
                    ci.validate(needs, "scope", self.jobs)
        for selector, jobs in [
            ("scope", []),
            ("scope", ["core", "core"]),
            ("scope", ["scope"]),
            ("scope", ["core (ubuntu-24.04)"]),
            ("scope", [""]),
            ("bad selector", ["core"]),
        ]:
            with (
                self.subTest(selector=selector, jobs=jobs),
                self.assertRaises(ValueError),
            ):
                ci.validate(self.fixture(), selector, jobs)

    def test_cli_handles_environment_input_and_repeated_job_options(self):
        for full in ["true", "false"]:
            with self.subTest(full=full):
                result = self.cli(
                    json.dumps(self.fixture(full)),
                    "--jobs",
                    "core",
                    "python",
                    "--jobs",
                    "desktop",
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(f"full={full}; all 3 jobs", result.stdout)

    def test_cli_rejects_absent_malformed_and_duplicate_json(self):
        for raw in [None, "", " \n", "{", "null", "[]", '{"scope":{},"scope":{}}']:
            with self.subTest(raw=raw):
                result = self.cli(raw, "--jobs", *self.jobs)
                self.assertEqual(result.returncode, 1)
                self.assertIn("CI result check failed:", result.stderr)
                self.assertEqual(result.stdout, "")

    def test_cli_failure_checks_remain_enabled_under_python_optimization(self):
        needs = self.fixture("false")
        needs["scope"]["result"] = "failure"
        result = self.cli(json.dumps(needs), "--jobs", *self.jobs, optimized=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("Selector 'scope'", result.stderr)
        needs = self.fixture()
        needs["python"]["result"] = "cancelled"
        result = self.cli(json.dumps(needs), "--jobs", *self.jobs, optimized=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("python: expected 'success', received 'cancelled'", result.stderr)


if __name__ == "__main__":
    unittest.main()
