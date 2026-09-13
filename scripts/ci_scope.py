"""Select native CI qualification without reducing manual release coverage.

Only pull requests, or pushes explicitly enabled with --scope-push, whose entire
diff contains website files or allowed documentation can omit native
qualification. Website tests still run separately. Package source, fixtures,
licenses, build tools and every unfamiliar path require the full matrix. All
other events, including manual and reusable release workflows, always require
it. Release workflows should leave --scope-push disabled.

The CLI reads committed changes from BASE...HEAD for PRs or BASE..HEAD for
opted-in pushes; both arguments must be complete immutable commit IDs. It prints
``full`` and ``reason`` as GitHub output lines and optionally appends them to
--github-output.
Missing history, invalid revisions and empty diffs select the full matrix. The
script never fetches, changes the checkout or reads the working-tree diff.
"""

from __future__ import annotations

import argparse
import re
import subprocess
from collections.abc import Iterable
from pathlib import Path, PurePosixPath
from typing import NamedTuple

ROOT_PROSE = frozenset({"README.md", "CONTRIBUTING.md", "AGENTS.md", "CHANGELOG.md"})
# This exact editorial inventory has no native/build consumer. Other CSV files
# remain data inputs and require full qualification, including adjacent files.
DOC_INVENTORY = "doc/documentation-audience.csv"
# These formats are prose, rendered figures, screenshots and static galleries.
# Do not add generic data formats: benchmark inputs also live under doc/.
DOC_PRESENTATION_SUFFIXES = frozenset(
    {
        ".md",
        ".mdx",
        ".rst",
        ".png",
        ".jpg",
        ".jpeg",
        ".webp",
        ".gif",
        ".svg",
        ".pdf",
        ".html",
        ".css",
    }
)
COMMIT_ID = re.compile(r"(?:[0-9a-fA-F]{40}|[0-9a-fA-F]{64})")


class Scope(NamedTuple):
    """Native qualification decision and its single-line explanation."""

    full: bool
    reason: str


def is_presentation_path(path: str) -> bool:
    """Return whether a normalized repository path can omit native checks.

    Website files retain their own build and browser checks. Outside website/,
    only named root prose, the exact audience inventory and doc/ presentation
    formats qualify. Malformed, absolute, traversing or control-character paths
    are never exempted. This does not inspect contents or make claims about
    documentation correctness.
    """
    if (
        not path
        or "\\" in path
        or ":" in path
        or any(
            ord(char) < 32 or ord(char) == 127 or 0xD800 <= ord(char) <= 0xDFFF
            for char in path
        )
        or any(part in {"", ".", ".."} for part in path.split("/"))
    ):
        return False
    if path in ROOT_PROSE or path == DOC_INVENTORY or path.startswith("website/"):
        return True
    return (
        path.startswith("doc/")
        and PurePosixPath(path).suffix in DOC_PRESENTATION_SUFFIXES
    )


def classify_paths(paths: Iterable[str]) -> Scope:
    """Require native qualification unless every changed path is allowlisted.

    Empty diffs require full qualification because they do not establish a
    documentation-only change. Reasons deliberately omit filenames so an
    unusual Git filename cannot inject additional GitHub output lines.
    """
    count = 0
    for path in paths:
        count += 1
        if not is_presentation_path(path):
            return Scope(True, "A changed path requires native qualification.")
    if count == 0:
        return Scope(True, "No changed paths were established.")
    return Scope(
        False,
        f"All {count} changed paths are allowed website or documentation files.",
    )


def changed_paths(
    base: str, head: str, repository: Path, *, merge_base: bool = True
) -> list[str]:
    """Read changed paths from complete commit IDs using a NUL-delimited diff.

    Compare HEAD with the merge base of BASE and HEAD, matching pull-request
    scope even when the target branch has advanced. Set merge_base=False for
    pushes to compare both commit trees directly, including removals from a
    divergent previous branch tip. Rename detection is off so both the removed
    path and added path affect selection. Raise ValueError for
    invalid IDs or malformed output; Git and filesystem failures propagate.
    Neither branch names, abbreviated IDs, tags nor working-tree changes are
    accepted as substitutes for the supplied immutable commits.
    """
    for commit in (base, head):
        if not COMMIT_ID.fullmatch(commit):
            raise ValueError("Expected a complete immutable commit ID")
        resolved = (
            subprocess.run(
                ["git", "rev-parse", "--verify", f"{commit}^{{commit}}"],
                cwd=repository,
                check=True,
                capture_output=True,
            )
            .stdout.decode("ascii")
            .strip()
        )
        if resolved != commit.lower():
            raise ValueError("Expected a commit object, not another object or ref")
    result = subprocess.run(
        [
            "git",
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--name-only",
            "-z",
            "--no-renames",
            f"{base}{'...' if merge_base else '..'}{head}",
            "--",
        ],
        cwd=repository,
        check=True,
        capture_output=True,
    ).stdout
    if not result:
        return []
    if not result.endswith(b"\0"):
        raise ValueError("Expected NUL-delimited changed paths")
    return [
        path.decode("utf-8", errors="surrogateescape")
        for path in result[:-1].split(b"\0")
    ]


def select_scope(
    event: str,
    base: str | None = None,
    head: str | None = None,
    *,
    force_full: bool = False,
    scope_push: bool = False,
    repository: Path | None = None,
) -> Scope:
    """Choose full qualification unless an eligible event has a verified diff.

    force_full always takes precedence. scope_push defaults to False and only
    enables comparison for push events, using the previous and new commit
    trees. Manual, scheduled, reusable and unknown events always require full
    qualification, even with scope_push=True and documentation-only commit IDs.
    Git failures select full qualification without changing repository state.
    """
    if force_full:
        return Scope(True, "Full native qualification was explicitly requested.")
    if event != "pull_request" and not (event == "push" and scope_push):
        return Scope(True, "This event requires full native qualification.")
    if not base or not head:
        return Scope(True, "Immutable base and head commits were not provided.")
    try:
        paths = changed_paths(
            base, head, repository or Path.cwd(), merge_base=event == "pull_request"
        )
    except (ValueError, OSError, subprocess.SubprocessError):
        return Scope(True, "The committed change scope could not be verified.")
    return classify_paths(paths)


def main() -> None:
    """Print the decision and optionally append GitHub Actions job outputs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--event", required=True, help="GitHub event name")
    parser.add_argument(
        "--base", help="Full immutable PR base or previous push commit ID"
    )
    parser.add_argument("--head", help="Full immutable PR head or new push commit ID")
    parser.add_argument(
        "--scope-push",
        action="store_true",
        help="Allow push event scope selection; leave disabled in release workflows",
    )
    parser.add_argument(
        "--force-full",
        action="store_true",
        help="Require all native qualification jobs",
    )
    parser.add_argument(
        "--github-output",
        type=Path,
        help="Append full and reason to this GitHub output file",
    )
    args = parser.parse_args()
    scope = select_scope(
        args.event,
        args.base,
        args.head,
        force_full=args.force_full,
        scope_push=args.scope_push,
    )
    output = f"full={str(scope.full).lower()}\nreason={scope.reason}\n"
    if args.github_output is not None:
        with args.github_output.open("a", encoding="utf-8") as destination:
            destination.write(output)
    print(output, end="")


if __name__ == "__main__":
    main()
