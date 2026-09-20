"""Validate version-specific CPython stable-ABI distribution contracts.

The stable ABI shares one native binary across GIL-enabled CPython 3.10–3.14;
NumPy compatibility is tested independently. This module uses Python 3.10's
standard library so the same checks run on every supported interpreter.
Version 0.2.12 ships three wheels; older ABI3 releases retain their four-wheel
inventory, including Intel macOS.
"""

import hashlib
import importlib
import importlib.metadata
import json
import re
import sys
import sysconfig
import zipfile
from email.parser import BytesParser
from pathlib import Path

PLATFORMS = {
    "ubuntu-24.04": frozenset({"manylinux_2_17_x86_64", "manylinux2014_x86_64"}),
    "macos-15": frozenset({"macosx_11_0_arm64"}),
    "macos-15-intel": frozenset({"macosx_10_12_x86_64"}),
    "windows-2025": frozenset({"win_amd64"}),
}


def python_version(version):
    """Convert a coordinated Cargo release version to its Python spelling."""
    match = re.fullmatch(r"(\d+\.\d+\.\d+)(?:-(alpha|beta|rc)\.(\d+))?", version)
    if not match:
        raise ValueError("Expected a coordinated release version")
    suffix = {"alpha": "a", "beta": "b", "rc": "rc"}
    return match[1] + (suffix[match[2]] + match[3] if match[2] else "")


def source_version():
    """Read the coordinated source version without requiring Python 3.11 TOML."""
    root = Path(__file__).resolve().parents[1]
    return json.loads((root / "js-rexafs/package.json").read_text())["version"]


def platforms_for_version(version):
    """Return required wheel platforms, preserving historical ABI3 inventories.

    Intel macOS is supported through 0.2.11. Version 0.2.12 and later require
    Apple Silicon macOS, Linux x64 and Windows x64. Prereleases use their numeric
    release version. Invalid version strings raise ValueError.
    """
    python_version(version)
    numeric = tuple(int(part) for part in version.split("-", 1)[0].split("."))
    return {runner: tags for runner, tags in PLATFORMS.items()
            if runner != "macos-15-intel" or numeric < (0, 2, 12)}


def identity(name, version):
    """Return the runner and expanded tags for one supported cp310-abi3 wheel.

    Wheel names must match the source version and existing native platform
    baselines exactly. Compressed manylinux aliases may be reordered, but no
    platform, architecture, Python baseline or ABI may be added silently.
    """
    prefix = f"rexafs-{python_version(version)}-cp310-abi3-"
    if not name.startswith(prefix) or not name.endswith(".whl"):
        raise ValueError(f"Expected a cp310-abi3 wheel for {version}: {name}")
    platforms = name[len(prefix):-4].split(".")
    if len(platforms) != len(set(platforms)):
        raise ValueError("Repeated wheel platform tag")
    for runner, expected in platforms_for_version(version).items():
        if set(platforms) == expected:
            return runner, {f"cp310-abi3-{platform}" for platform in platforms}
    raise ValueError(f"Unsupported wheel platform tags: {name}")


def inventory(names, version):
    """Require exactly one ABI3 wheel per platform supported by this version."""
    runners = [identity(name, version)[0] for name in names]
    platforms = platforms_for_version(version)
    if len(runners) != len(platforms) or set(runners) != set(platforms):
        raise ValueError(f"Expected exactly {len(platforms)} ABI3 wheels, one per release platform")


def check_wheel(path, version, platform=None):
    """Verify filename, metadata, typed package files and native extension name.

    Return the SHA-256 of the checked bytes. This checks the declared ABI and
    package identity; native import and API tests must still run on each OS.
    No archive entries are extracted and no files are changed.
    """
    runner, tags = identity(path.name, version)
    if platform is not None and runner != platform:
        raise ValueError(f"Wheel belongs to {runner}, not {platform}")
    dist_info = f"rexafs-{python_version(version)}.dist-info"
    with zipfile.ZipFile(path) as wheel:
        names = wheel.namelist()
        if len(names) != len(set(names)):
            raise ValueError("Wheel contains duplicate archive entries")
        metadata = BytesParser().parsebytes(wheel.read(f"{dist_info}/METADATA"))
        headers = BytesParser().parsebytes(wheel.read(f"{dist_info}/WHEEL"))
        if metadata.get_all("Name") != ["rexafs"] or metadata.get_all("Version") != [python_version(version)]:
            raise ValueError("Wheel metadata does not match the release identity")
        if metadata.get("Requires-Python", "").replace(" ", "") != ">=3.10,<3.15":
            raise ValueError("Wheel must retain the supported CPython 3.10–3.14 range")
        if metadata.get_all("Requires-Dist") != ["numpy>=1.23"]:
            raise ValueError("Wheel must retain the tested NumPy dependency floor")
        actual_tags = headers.get_all("Tag", [])
        if set(actual_tags) != tags or len(actual_tags) != len(tags):
            raise ValueError("Wheel metadata tags differ from its filename")
        if headers.get("Root-Is-Purelib") != "false":
            raise ValueError("Expected a native platform wheel")
        extension = "rexafs/_core.pyd" if runner == "windows-2025" else "rexafs/_core.abi3.so"
        native = {name for name in names if name.startswith("rexafs/_core.")}
        if native != {extension}:
            raise ValueError("Expected exactly one stable-ABI native extension")
        required = {"rexafs/__init__.py", "rexafs/__init__.pyi", "rexafs/io.py", "rexafs/io.pyi", "rexafs/py.typed"}
        if not required <= set(names):
            raise ValueError("Wheel is missing Python API or editor files")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_installed(path, version, numpy_version=None):
    """Require this wheel's exact package bytes in the active CPython environment.

    Import the installed package, compare its Python, native and editor files
    with the downloaded wheel, and reject source-tree imports or free-threaded
    runtimes. Optionally require an exact NumPy floor. Return runtime evidence;
    callers run the numerical API suite separately using this interpreter.
    """
    if sys.implementation.name != "cpython" or sysconfig.get_config_var("Py_GIL_DISABLED"):
        raise ValueError("This qualification covers GIL-enabled CPython only")
    if not (3, 10) <= sys.version_info[:2] <= (3, 14):
        raise ValueError("Unsupported qualification interpreter")
    package = importlib.import_module("rexafs")
    core = importlib.import_module("rexafs._core")
    numpy = importlib.import_module("numpy")
    distribution = importlib.metadata.distribution("rexafs")
    expected_version = python_version(version)
    # Native __version__ comes from CARGO_PKG_VERSION; wheel metadata uses PEP 440.
    if distribution.version != expected_version or package.__version__ != version:
        raise ValueError("Installed package version differs from the checked wheel")
    if numpy_version is not None and numpy.__version__ != numpy_version:
        raise ValueError("Installed NumPy differs from the requested minimum")
    package_root = Path(package.__file__).resolve().parent
    site_roots = {Path(sysconfig.get_path(key)).resolve() for key in ("purelib", "platlib")}
    if package_root.parent not in site_roots:
        raise ValueError("rexafs must import from this interpreter's installed site-packages")
    with zipfile.ZipFile(path) as wheel:
        for name in wheel.namelist():
            if name.startswith("rexafs/") and not name.endswith("/"):
                installed = package_root / name.removeprefix("rexafs/")
                if not installed.is_file() or installed.read_bytes() != wheel.read(name):
                    raise ValueError(f"Installed file differs from the qualified wheel: {name}")
        extensions = [name for name in wheel.namelist() if name.startswith("rexafs/_core.")]
        if len(extensions) != 1 or Path(core.__file__).resolve() != package_root / Path(extensions[0]).name:
            raise ValueError("Imported native extension is not the checked wheel's extension")
    return {"python": sys.version.split()[0], "numpy": numpy.__version__,
            "rexafs": package.__version__, "import": str(package_root), "native": core.__file__}
