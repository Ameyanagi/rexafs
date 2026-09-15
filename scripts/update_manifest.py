"""Record files owned by a desktop package for verified in-place upgrades.

The enclosing release archive is authenticated using GitHub's SHA-256 metadata.
This inventory is not a publisher signature. It distinguishes package files
from user files so the updater can preserve the latter without overwriting them.
"""

import hashlib
import json
from pathlib import Path


def write_manifest(bundle: Path) -> None:
    files = []
    for path in sorted(bundle.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"Desktop updates do not support symbolic links: {path}")
        if path.is_file() and path.name != "update-manifest.json":
            with path.open("rb") as source:
                digest = hashlib.file_digest(source, "sha256").hexdigest()
            files.append(
                {"path": path.relative_to(bundle).as_posix(), "sha256": digest}
            )
    (bundle / "update-manifest.json").write_text(
        json.dumps({"version": 1, "files": files}, indent=2) + "\n", encoding="utf-8"
    )
