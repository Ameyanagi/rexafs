"""Check ABI3 wheel identity, the version's platform inventory, or installed bytes."""

import argparse
import json
from pathlib import Path

from python_wheels import PLATFORMS, check_installed, check_wheel, inventory, source_version


def main():
    """Check existing wheel files; print their hashes and optional runtime evidence."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--version", default=source_version())
    parser.add_argument("--platform", choices=PLATFORMS)
    parser.add_argument("--inventory", action="store_true")
    parser.add_argument("--installed", action="store_true")
    parser.add_argument("--numpy-version")
    args = parser.parse_args()
    paths = sorted(args.directory.rglob("*.whl"))
    if args.inventory:
        inventory([path.name for path in paths], args.version)
    elif len(paths) != 1:
        raise ValueError("Expected exactly one downloaded wheel")
    if args.installed and len(paths) != 1:
        raise ValueError("Installed checks require exactly one wheel")
    for path in paths:
        evidence = {"wheel": path.name, "sha256": check_wheel(path, args.version, args.platform)}
        if args.installed:
            evidence.update(check_installed(path, args.version, args.numpy_version))
        print(json.dumps(evidence, sort_keys=True))


if __name__ == "__main__":
    main()
