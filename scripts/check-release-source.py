"""Resolve a source tag and require its successful, manually dispatched build.

The CLI takes TAG RUN_ID, reads the tag's Cargo version using Git, and queries
the run through gh in GITHUB_REPOSITORY. On success it appends commit, version,
tag and the source tag's Python ABI profile to GITHUB_OUTPUT. It does not move tags, build
packages, or inspect artifact bytes; the publication workflow performs the
separate artifact checks after this source validation.
"""
import json
import os
import re
import subprocess
import sys
import tomllib


def python_abi(cargo):
    """Select the wheel contract from the immutable source's PyO3 features.

    Historical tags retain their manifest-defined per-interpreter wheels.
    Enabling abi3-py310 selects four shared wheels; other stable-ABI baselines
    require an explicit qualification and publishing contract update.
    """
    features = cargo["dependencies"]["pyo3"].get("features", [])
    stable = {feature for feature in features if feature.startswith("abi3")}
    if stable == {"abi3-py310"}:
        return "abi3-py310"
    if stable:
        raise ValueError("Unsupported source Python stable-ABI feature selection")
    return "per-interpreter"


def validate(run, tag, commit, version):
    """Require a successful release-build.yml dispatch for this exact source.

    run is the GitHub Actions run JSON; tag must equal ``v`` plus the coordinated
    Cargo version. The head SHA, tag name, event, workflow path and conclusion
    must all match. Pull-request builds cannot qualify a release, even if they
    tested the same commit. Raises ValueError on a mismatch and otherwise
    returns None without network access or mutation.
    """
    if not re.fullmatch(r"v\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?", tag):
        raise ValueError("Expected a coordinated release tag")
    if tag != f"v{version}":
        raise ValueError("Tag does not match the source version")
    expected = {"conclusion": "success", "head_sha": commit,
                "head_branch": tag, "event": "workflow_dispatch",
                "path": ".github/workflows/release-build.yml"}
    for key, value in expected.items():
        if run.get(key) != value:
            raise ValueError(f"Release build has an unexpected {key}")


if __name__ == "__main__":
    tag, run_id = sys.argv[1:]
    if not re.fullmatch(r"v\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?", tag) or not run_id.isdecimal():
        raise SystemExit("Invalid release tag or build run ID")
    commit = subprocess.check_output(["git", "rev-parse", f"refs/tags/{tag}^{{commit}}"], text=True).strip()
    cargo = subprocess.check_output(["git", "show", f"{commit}:Cargo.toml"], text=True)
    version = tomllib.loads(cargo)["workspace"]["package"]["version"]
    binding = subprocess.check_output(["git", "show", f"{commit}:py-rexafs/Cargo.toml"], text=True)
    abi = python_abi(tomllib.loads(binding))
    run = json.loads(subprocess.check_output([
        "gh", "api", f"repos/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{run_id}"]))
    validate(run, tag, commit, version)
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"commit={commit}\nversion={version}\ntag={tag}\npython_abi={abi}\n")
    print(f"Qualified {tag} at {commit}, build {run_id}")
