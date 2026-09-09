"""Run every release-tool regression script, including their hyphenated names."""
from pathlib import Path
import subprocess
import sys


def main():
    root = Path(__file__).resolve().parents[1]
    suites = sorted((root / "scripts").glob("test-*.py"))
    if not suites:
        raise SystemExit("No tooling test suites found")
    failed = []
    for suite in suites:
        print(f"Running {suite.name}", flush=True)
        if subprocess.run([sys.executable, str(suite)], cwd=root).returncode:
            failed.append(suite.name)
    if failed:
        raise SystemExit("Failed suites: " + ", ".join(failed))
    print(f"All {len(suites)} tooling suites passed")


if __name__ == "__main__":
    main()
