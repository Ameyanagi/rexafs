"""Stage desktop-only GitHub downloads from a qualified release manifest."""
import argparse
import hashlib
from pathlib import Path
import re
import shutil


TARGETS = {
    "aarch64-apple-darwin": ".zip",
    "x86_64-apple-darwin": ".zip",
    "x86_64-pc-windows-msvc": ".zip",
    "x86_64-unknown-linux-gnu": ".tar.gz",
}


def desktop_names(entries, version):
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?", version):
        raise ValueError("Expected a coordinated release version")
    required, allowed = set(), set()
    for target, extension in TARGETS.items():
        stem = f"rexafs-{version}-{target}"
        required.update((stem + extension, stem + extension + ".sha256"))
        if target.endswith("windows-msvc"):
            required.update(stem + suffix for suffix in (
                "-setup.exe", "-setup.exe.sha256", "-setup.build.json", "-setup.exe.qualification.json"))
        if target.endswith("apple-darwin"):
            installers = {stem + suffix for suffix in (".dmg", ".dmg.sha256", ".dmg.json")}
            if installers & entries.keys() and not installers <= entries.keys():
                raise ValueError("Incomplete Mac installer evidence")
            allowed.update(installers)
    if not required <= entries.keys():
        raise ValueError("Missing required desktop archives, installers, or checksums")
    return (required | allowed) & entries.keys()


def stage(artifacts, manifest, output, version):
    entries = {}
    for line in manifest.read_text().splitlines():
        digest, name = line.split("  ", 1)
        if not re.fullmatch(r"[0-9a-f]{64}", digest) or "/" in name or "\\" in name or name in entries:
            raise ValueError("Invalid or duplicate manifest entry")
        entries[name] = digest
    names = desktop_names(entries, version)
    paths = {}
    for path in artifacts.rglob("*"):
        if path.is_file() and path.name in names:
            if path.name in paths:
                raise ValueError("Duplicate desktop artifact: " + path.name)
            with path.open("rb") as stream:
                if hashlib.file_digest(stream, "sha256").hexdigest() != entries[path.name]:
                    raise ValueError("Desktop artifact differs from the qualified build: " + path.name)
            paths[path.name] = path
    if paths.keys() != names:
        raise ValueError("Missing desktop artifact files")
    # Refuse to mix this release with files from an earlier staging attempt.
    output.mkdir(parents=True, exist_ok=False)
    for name, path in paths.items():
        shutil.copy2(path, output / name)
    (output / "SHA256SUMS").write_text("".join(f"{entries[name]}  {name}\n" for name in sorted(names)))
    return len(names)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("version")
    args = parser.parse_args()
    print(f"Staged {stage(args.artifacts, args.manifest, args.output, args.version)} qualified desktop downloads")
