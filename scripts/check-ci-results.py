"""Require every selected CI job to finish with its declared result.

Pass GitHub Actions' ``toJSON(needs)`` through ``CI_NEEDS_JSON`` and identify
the selector with ``--selector`` and its gated jobs with ``--jobs``. The selector
must succeed and emit ``outputs.full`` as exactly ``"true"`` or ``"false"``.
A full check requires successful jobs; a documentation-only check requires
those jobs to be skipped. Neither a failed selector nor a skipped required job
can produce a successful aggregate check.

This checks GitHub's aggregate result for each matrix job, not individual
matrix entries or release artifacts. Release qualification remains separate.
"""

import argparse
import json
import os
import re
import sys
from collections.abc import Sequence


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    """Decode an object without silently accepting repeated JSON keys."""
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def validate(needs: object, selector: str, jobs: Sequence[str]) -> bool:
    """Return whether full CI ran, or reject invalid dependency results.

    ``needs`` must contain exactly the selector and the listed job identifiers.
    Identifiers refer to workflow job keys, not display names or matrix labels.
    Every listed job must have result ``success`` when ``full`` is ``"true"``
    and ``skipped`` when it is ``"false"``. Unexpected jobs are rejected so that
    an added dependency cannot fail without the aggregate noticing it.
    A non-object ``needs`` raises TypeError; invalid fields raise ValueError.
    The input object is not modified.
    """
    names = [selector, *jobs]
    if not jobs:
        raise ValueError("At least one gated job is required")
    for name in names:
        if not isinstance(name, str) or not re.fullmatch(
            r"[A-Za-z_][A-Za-z0-9_-]*", name
        ):
            raise ValueError(f"Invalid workflow job identifier: {name!r}")
    if len(set(names)) != len(names):
        raise ValueError("Selector and gated job identifiers must be unique")
    if not isinstance(needs, dict):
        raise TypeError("CI_NEEDS_JSON must describe a JSON object")
    expected = set(names)
    actual = set(needs)
    if actual != expected:
        missing = sorted(expected - actual)
        unexpected = sorted(str(name) for name in actual - expected)
        raise ValueError(
            f"CI dependency inventory differs: missing={missing}, unexpected={unexpected}"
        )
    scope = needs[selector]
    if not isinstance(scope, dict) or scope.get("result") != "success":
        raise ValueError(f"Selector {selector!r} must finish with result 'success'")
    outputs = scope.get("outputs")
    if not isinstance(outputs, dict) or outputs.get("full") not in ("true", "false"):
        raise ValueError(
            f"Selector {selector!r} must emit outputs.full as 'true' or 'false'"
        )
    full = outputs["full"] == "true"
    required = "success" if full else "skipped"
    failures = []
    for name in jobs:
        job = needs[name]
        result = job.get("result") if isinstance(job, dict) else None
        if result != required:
            failures.append(f"{name}: expected {required!r}, received {result!r}")
    if failures:
        raise ValueError(
            "CI job results differ from the selector: " + "; ".join(failures)
        )
    return full


def main() -> int:
    """Read dependency results from the environment and print a brief verdict."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selector", required=True, help="Job that emits outputs.full")
    parser.add_argument(
        "--jobs",
        nargs="+",
        action="append",
        required=True,
        help="Gated workflow job identifiers; this option may be repeated",
    )
    args = parser.parse_args()
    jobs = [name for group in args.jobs for name in group]
    try:
        raw = os.environ.get("CI_NEEDS_JSON")
        if raw is None or not raw.strip():
            raise ValueError("CI_NEEDS_JSON is required and must not be empty")
        needs = json.loads(raw, object_pairs_hook=unique_object)
        full = validate(needs, args.selector, jobs)
    except (TypeError, ValueError) as error:
        print(f"CI result check failed: {error}", file=sys.stderr)
        return 1
    action = "passed" if full else "were intentionally skipped"
    print(
        f"CI selector {args.selector}: full={str(full).lower()}; all {len(jobs)} jobs {action}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
