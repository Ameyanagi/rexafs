"""Download the FEFF10 command-line helper that Windows desktop packages bundle.

The Windows desktop is an MSVC build and cannot link the MinGW FEFF10 archive.
It runs FEFF10 through the upstream feff10-rs command-line executable instead,
which re-executes itself as one fresh process per FEFF stage. This module
downloads that executable and its MinGW runtime libraries from the pinned
feff10-rs release, verifies every SHA-256 against the values recorded here,
and writes them to a destination directory. Nothing is executed.

Usage: python scripts/feff10_worker.py DESTINATION
"""
import hashlib
import sys
import urllib.request
from pathlib import Path

RELEASE = "v0.2.3"
BASE_URL = f"https://github.com/Ameyanagi/feff10-rs/releases/download/{RELEASE}/"
# Published asset name -> (bundled file name, SHA-256 from the release's sha256sums.txt).
ASSETS = {
    "feff10-windows-x86_64.exe": (
        "feff10-rs.exe",
        "351e4fccad1896afb44d84cb5169afe41d6f3ec07d4af0a87acb035737088cd6",
    ),
    "libgcc_s_seh-1.dll": (
        "libgcc_s_seh-1.dll",
        "278f1101f2371c58b6b412a3206718e22343940a5164aceb4fb75f2bf3b38ed4",
    ),
    "libgfortran-5.dll": (
        "libgfortran-5.dll",
        "c03937b7e6be0d37c3f52edf742b20bcbf52d03e5419dc26498e7b55d75f1b28",
    ),
    "libquadmath-0.dll": (
        "libquadmath-0.dll",
        "3408cb0853bef06dcf106148f9bbaed3db904efe19490d4322aba74112059d5e",
    ),
    "libwinpthread-1.dll": (
        "libwinpthread-1.dll",
        "4869aa8ab2e7e19ee43109b8832749aa1cd9c6e5302475439eae42d0b60b969f",
    ),
}
HELPER_EXECUTABLE = "feff10-rs.exe"


def download(asset: str, expected_sha256: str) -> bytes:
    """Return the verified bytes of one release asset."""
    with urllib.request.urlopen(BASE_URL + asset, timeout=120) as response:
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"{asset}: SHA-256 {digest} does not match the recorded {expected_sha256}")
    return data


def install_helper(destination: Path) -> Path:
    """Write the verified helper executable and runtime libraries; return the executable path."""
    destination.mkdir(parents=True, exist_ok=True)
    for asset, (name, expected) in ASSETS.items():
        (destination / name).write_bytes(download(asset, expected))
    return destination / HELPER_EXECUTABLE


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    print(install_helper(Path(sys.argv[1]).resolve()))
