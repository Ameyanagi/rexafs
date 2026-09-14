"""Plan immutable nightly identities and publish only qualified signed Mac archives."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
from zipfile import ZipFile
from macos_installer import qualify_installer

TARGETS = {"aarch64-apple-darwin", "x86_64-apple-darwin"}
REPOSITORY = "Ameyanagi/rexafs"


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def require_dev(env):
    """Require a push/manual run on this repository's development branch.

    Read the supplied environment mapping without changing it. Other refs,
    repositories or event types raise ValueError before publication work starts.
    """
    if (env.get("GITHUB_REF") != "refs/heads/dev" or env.get("GITHUB_REPOSITORY") != REPOSITORY
            or env.get("GITHUB_EVENT_NAME") not in {"push", "workflow_dispatch"}):
        raise ValueError("Nightly publication requires this repository's dev branch")


def build_plan(run, version, commit, run_id):
    """Return a nightly identity tied to one dev-branch Actions run.

    Validate run ID, source SHA, branch and event, then derive the dated tag and
    UTC timestamp from the run's creation time. Rerunning the same run preserves
    this identity even after midnight. The plan does not establish that builds
    or signing have succeeded; qualify performs the artifact checks later.
    """
    if (str(run.get("id")) != run_id or run.get("head_sha") != commit
            or run.get("head_branch") != "dev" or run.get("event") not in {"push", "workflow_dispatch"}):
        raise ValueError("Nightly run does not match the dev-branch source")
    # Run creation time is stable across reruns, including reruns after midnight.
    created = datetime.fromisoformat(run["created_at"].replace("Z", "+00:00")).astimezone(timezone.utc)
    return dict(version=version, commit=commit, tag=f"nightly-{created:%Y%m%d}-{run_id}", built_at=created.strftime("%Y-%m-%dT%H:%M:%SZ"))


def qualify(archives, version, commit, run_id, tag):
    """Return hashes only when both Mac architectures have complete evidence.

    Require matching source/signing metadata, checksums, the Nightly app name
    and qualified DMG sidecars for each ZIP. This reads files and evidence;
    it does not rerun graphical tests, Apple's checks, or publish anything.
    Missing/duplicate targets and inconsistent provenance raise ValueError.
    """
    if not re.fullmatch(r"nightly-\d{8}-\d+", tag) or not tag.endswith("-" + run_id):
        raise ValueError("Nightly tag does not identify this run")
    targets = set()
    hashes = {}
    for archive in archives:
        with ZipFile(archive) as zipped:
            metadata = json.loads(zipped.read(archive.stem + "/build.json"))
            target = metadata.get("target")
            expected = dict(version=version, commit=commit, github_run_id=run_id,
                            signing_run_id=run_id, signing_tools_commit=commit,
                            channel="nightly", release_tag=tag, dirty=False, signed=True, notarized=True)
            if any(metadata.get(key) != value for key, value in expected.items()):
                raise ValueError("Nightly archive has unqualified source/signing metadata")
            if not metadata.get("notarization_id") or not metadata.get("apple_team_id"):
                raise ValueError("Nightly archive is missing notarization provenance")
            if target not in TARGETS or target in targets:
                raise ValueError("Unexpected or duplicate nightly target")
            if archive.name != f"rexafs-{version}-{target}.zip":
                raise ValueError("Unexpected nightly archive name")
            if archive.stem + "/rexafs Nightly.app/Contents/MacOS/rexafs" not in zipped.namelist():
                raise ValueError("Nightly app must coexist with the stable app")
            targets.add(target)
        digest = sha256(archive)
        sidecar = Path(str(archive) + ".sha256")
        if sidecar.read_text().strip() != f"{digest}  {archive.name}":
            raise ValueError("Nightly archive checksum differs from its signed artifact")
        hashes[archive.name] = digest
        hashes[sidecar.name] = sha256(sidecar)
        hashes.update(qualify_installer(archive.with_suffix(".dmg"), metadata, archive))
    if targets != TARGETS:
        raise ValueError("Both qualified Mac architectures are required")
    return hashes


def get_draft_release(tag):
    """Read the matching draft through gh and return its full release JSON.

    Resolve the draft's database ID before calling the REST API because the
    release-by-tag endpoint does not resolve these drafts. Reject a public or
    differently named release; this operation does not modify GitHub state.
    """
    # GitHub's release-by-tag endpoint does not resolve the draft created below.
    # gh release view resolves drafts; use its database ID for the API asset data.
    resolved = json.loads(subprocess.check_output([
        "gh", "release", "view", tag, "--repo", REPOSITORY,
        "--json", "databaseId,isDraft,tagName"], text=True))
    release_id = resolved.get("databaseId")
    if (resolved.get("tagName") != tag or resolved.get("isDraft") is not True
            or type(release_id) is not int or release_id <= 0):
        raise ValueError("Expected an existing draft for this nightly tag")
    remote = json.loads(subprocess.check_output([
        "gh", "api", f"repos/{REPOSITORY}/releases/{release_id}"], text=True))
    if remote.get("id") != release_id or remote.get("tag_name") != tag or remote.get("draft") is not True:
        raise ValueError("Resolved release is not the expected nightly draft")
    return remote


def publish(directory, version, commit, run_id, tag):
    """Publish qualified nightly desktop artifacts, preserving an existing tag.

    Validate both architectures and installers before creating or resuming a
    draft. Write local notes/checksums, upload the qualified files, verify remote
    asset digests, then make the draft public. Refuse to replace a public nightly
    or reuse a tag pointing at another commit. This performs GitHub mutations;
    it does not publish registry packages or rebuild executables.
    """
    archives = sorted(directory.rglob("*.zip"))
    hashes = qualify(archives, version, commit, run_id, tag)
    installers = list(directory.rglob("*.dmg"))
    if sorted(p.name for p in installers) != sorted(p.with_suffix(".dmg").name for p in archives):
        raise ValueError("Unexpected or duplicate nightly installers")
    # A tag is immutable, including across reruns that resume a draft upload.
    ref = subprocess.run(["gh", "api", f"repos/{REPOSITORY}/git/ref/tags/{tag}"], capture_output=True, text=True)
    if ref.returncode == 0:
        obj = json.loads(ref.stdout)["object"]
        if obj.get("type") != "commit" or obj.get("sha") != commit:
            raise ValueError("Existing nightly tag points to another commit")
    existing = subprocess.run(["gh", "release", "view", tag, "--repo", REPOSITORY, "--json", "isDraft"], capture_output=True, text=True)
    if existing.returncode == 0 and not json.loads(existing.stdout)["isDraft"]:
        raise ValueError("This nightly is already public and will not be replaced; dispatch a new run")
    manifest = directory / "SHA256SUMS"
    manifest.write_text("".join(f"{hashes[name]}  {name}\n" for name in sorted(hashes)))
    notes = directory / "nightly-notes.md"
    notes.write_text(f"""Nightly desktop build from `{commit}` (library version {version}).

These prerelease builds include the newest changes on dev. They pass automated core/desktop regression tests and package self-checks; they do not receive a separate manual graphical audit every night.

Both Mac apps and DMG installers are signed with Developer ID, notarized, stapled and checked with Gatekeeper. Open the DMG, drag `rexafs Nightly.app` onto Applications, and eject the disk image. Nightly can coexist with Stable. Save your project before switching builds. ZIP archives remain available for the existing updater.

Use Updates → Nightly in the application to discover this channel. Stable remains the default. This run does not publish nightly packages to crates.io, PyPI or npm.

Build and signing provenance is in each archive's `build.json` and each installer's `.dmg.json` sidecar; checksums are in `SHA256SUMS`. Installation is smoke-tested from a fresh copy of the mounted DMG. Workflow: https://github.com/{REPOSITORY}/actions/runs/{run_id}
""")
    if existing.returncode != 0:
        subprocess.run(["gh", "release", "create", tag, "--repo", REPOSITORY, "--target", commit,
                        "--draft", "--prerelease", "--latest=false", "--title", "rexafs " + tag,
                        "--notes-file", str(notes)], check=True)
    assets = [path for path in directory.rglob("*") if path.is_file() and path.name in hashes] + [manifest]
    subprocess.run(["gh", "release", "upload", tag, "--repo", REPOSITORY, "--clobber", *map(str, assets)], check=True)
    remote = get_draft_release(tag)
    remote_hashes = {asset["name"]: asset.get("digest") for asset in remote["assets"]}
    hashes[manifest.name] = sha256(manifest)
    if remote_hashes != {name: "sha256:" + digest for name, digest in hashes.items()}:
        raise ValueError("Uploaded release checksums do not match; leaving the release as a draft")
    subprocess.run(["gh", "release", "edit", tag, "--repo", REPOSITORY, "--draft=false", "--prerelease", "--latest=false"], check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["plan", "publish"])
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--tag")
    args = parser.parse_args()
    require_dev(os.environ)
    root = Path(__file__).resolve().parents[1]
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if commit != os.environ["GITHUB_SHA"]:
        raise SystemExit("Checkout does not match the workflow source")
    run_id = os.environ["GITHUB_RUN_ID"]
    if not run_id.isdecimal():
        raise SystemExit("Invalid run ID")
    if args.command == "plan":
        run = json.loads(subprocess.check_output(["gh", "api", f"repos/{REPOSITORY}/actions/runs/{run_id}"], text=True))
        plan = build_plan(run, version, commit, run_id)
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            for key, value in plan.items():
                output.write(f"{key}={value}\n")
    else:
        if not args.directory or not args.tag:
            raise SystemExit("Publish requires a directory and immutable nightly tag")
        publish(args.directory, version, commit, run_id, args.tag)
