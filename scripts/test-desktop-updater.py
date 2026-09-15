"""Qualify the real Windows/Linux update helper on disposable CI installations.

The test stages an explicit transaction from the locally built package. It does
not contact releases or bypass downloaded-asset verification in the GUI. It tests
parent-exit handoff, replacement, recovery launch, and Windows installer rollback;
Rust tests separately exercise release selection, archive checks and cancellation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from update_manifest import write_manifest


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def canonical(path):
    value = str(path.resolve(strict=True))
    if os.name == "nt" and not value.startswith("\\\\?\\"):
        return "\\\\?\\UNC\\" + value[2:] if value.startswith("\\\\") else "\\\\?\\" + value
    return value


def wait_until(predicate, description, timeout=120):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise RuntimeError(f"Timed out: {description}")


def close_test_gui(root, target, output):
    receipt = root / "launched-pid.txt"
    wait_until(receipt.is_file, "recovery process receipt")
    pid = int(receipt.read_text())
    assert pid > 1
    executable = target / ("rexafs.exe" if os.name == "nt" else "rexafs")
    if os.name == "nt":
        # Fixed command plus an integer PID; paths are never shell-interpolated.
        actual = subprocess.check_output(["powershell", "-NoProfile", "-NonInteractive", "-Command",
                                          f"(Get-Process -Id {pid} -ErrorAction Stop).Path"], text=True).strip()
        assert Path(actual).resolve() == executable.resolve(), actual
        wait_until(lambda: subprocess.check_output(["powershell", "-NoProfile", "-NonInteractive", "-Command",
            f"(Get-Process -Id {pid} -ErrorAction Stop).MainWindowHandle"], text=True).strip() != "0", "reopened Windows window", 60)
        subprocess.run(["taskkill", "/PID", str(pid), "/T", "/F"], check=True, capture_output=True)
    else:
        assert Path(f"/proc/{pid}/exe").resolve() == executable.resolve()
        def window_id():
            result = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(pid), "--name", "rexafs"], capture_output=True, text=True)
            return result.stdout.strip().splitlines()[0] if result.returncode == 0 and result.stdout.strip() else None
        wait_until(window_id, "reopened Linux window", 60)
        time.sleep(2)
        subprocess.run(["import", "-window", window_id(), str(output / "reopened.png")], check=True)
        os.kill(pid, signal.SIGTERM)
        wait_until(lambda: not Path(f"/proc/{pid}/exe").exists(), "test GUI exit")


def exercise(source, target, project, output, setup=None, failure=False):
    output.mkdir(parents=True, exist_ok=True)
    name = "rexafs.exe" if os.name == "nt" else "rexafs"
    identity = json.loads((source / "build.json").read_text())
    sentinel = target / "my analysis 日本語.rxs"
    shutil.copy2(project, sentinel)
    original_user_hash = digest(sentinel)
    before = {path.relative_to(target).as_posix(): digest(path) for path in target.rglob("*") if path.is_file()}
    directory = tempfile.mkdtemp(prefix=".rexafs-update-", dir=target.parent)
    root = Path(directory)
    incoming = root / "incoming"
    shutil.copytree(source, incoming)
    helper_dir = root / "helper"
    helper_dir.mkdir()
    shutil.copy2(target / name, helper_dir / name)
    for dll in target.glob("*.dll"):
        shutil.copy2(dll, helper_dir / dll.name)
    recovery = root / "Update recovery.rxs"
    shutil.copy2(project, recovery)
    if setup:
        shutil.copy2(setup, root / "setup.exe")
        if failure:
            # Valid preflight package; Setup installs different known bytes.
            # This forces post-install verification to exercise real rollback.
            with (incoming / "README.txt").open("a") as stream:
                stream.write("\nDeliberate qualification mismatch.\n")
            write_manifest(incoming)
    elif failure:
        (incoming / "README.txt").write_text("Deliberately damaged download")
    parent = subprocess.Popen([sys.executable, "-c", "import sys; sys.stdin.buffer.read()"], stdin=subprocess.PIPE)
    plan = {"target": canonical(target), "parent_pid": parent.pid,
            "previous_hash": digest(target / name), "manifest_hash": digest(incoming / "update-manifest.json"),
            "tag": identity["release_tag"], "channel": identity["channel"],
            "setup_hash": digest(root / "setup.exe") if setup else None}
    (root / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    with (root / "install.log").open("w") as log:
        helper = subprocess.Popen([str(helper_dir / name), "--finish-update", str(root)], cwd=root,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        try:
            wait_until(lambda: (root / "ready").is_file() or helper.poll() is not None, "helper readiness")
            assert helper.poll() is None, (root / "install.log").read_text()
            assert not (root / "previous").exists(), "App changed while the parent was running"
            parent.stdin.close()
            parent.wait(timeout=10)
            helper.wait(timeout=300)
            result = (root / "result.txt").read_text() if (root / "result.txt").exists() else (root / "install.log").read_text()
            assert (helper.returncode != 0) == failure, result
            close_test_gui(root, target, output)
            assert digest(sentinel) == original_user_hash
            if failure:
                after = {path.relative_to(target).as_posix(): digest(path) for path in target.rglob("*") if path.is_file()}
                assert after == before, "Failed update changed the original installation"
            else:
                assert (root / "previous" / name).is_file()
                for entry in json.loads((source / "update-manifest.json").read_text())["files"]:
                    assert digest(target / entry["path"]) == entry["sha256"], entry["path"]
            assert digest(recovery) == digest(project)
            for filename in ["install.log", "setup.log", "result.txt", "reopened.log"]:
                if (root / filename).exists():
                    shutil.copy2(root / filename, output / filename)
            (output / "checks.json").write_text(json.dumps({"target": identity["target"], "installer": bool(setup),
                "expected_failure": failure, "checks": ["wait-for-parent", "recovery-launch", "preserve-user-project",
                "restore-original-files" if failure else "install-verified-payload"], "result": result}, indent=2) + "\n")
        finally:
            if parent.poll() is None:
                parent.stdin.close()
                parent.wait(timeout=10)
            if helper.poll() is None:
                # Do not kill an active installer and race its file writes.
                raise RuntimeError(f"Updater still running; retain transaction for diagnosis: {root}")
    shutil.rmtree(root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--installed", type=Path)
    parser.add_argument("--setup", type=Path)
    parser.add_argument("--output", type=Path, default=Path("target/update-smoke"))
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Run this native installation test only on an ephemeral CI runner")
    source = args.bundle.resolve(strict=True)
    project = Path(__file__).resolve().parents[1] / "crates/rexafs-gui/tests/fixtures/projects/rexafs-0.2.8-embedded.rxs"
    if args.installed:
        target = args.installed.resolve(strict=True)
        runner_temp = Path(os.environ["RUNNER_TEMP"]).resolve(strict=True)
        if not target.is_relative_to(runner_temp) or not args.setup:
            parser.error("Installed qualification must use the disposable installer-test directory and matching setup")
        for failure in [False, True]:
            exercise(source, target, project, args.output / ("installer-rollback" if failure else "installer-update"), args.setup.resolve(strict=True), failure)
    else:
        directory = tempfile.mkdtemp(prefix="rexafs portable update ")
        for failure in [False, True]:
            target = Path(directory) / ("failure 日本語" if failure else "success 日本語")
            shutil.copytree(source, target)
            with (target / "README.txt").open("a") as stream:
                stream.write("\nPrevious package fixture.\n")
            write_manifest(target)
            exercise(source, target, project, args.output / ("portable-rejection" if failure else "portable-update"), failure=failure)
        shutil.rmtree(directory)
    print("Native desktop updater checks passed.")


if __name__ == "__main__":
    main()
