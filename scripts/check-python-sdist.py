"""Check source archive licenses and, when requested, the stable-ABI build feature."""
import argparse
from email.parser import BytesParser
from pathlib import PurePosixPath
import tarfile
import tomllib


def check(archive, require_abi3=False):
    """Verify the existing License-File declarations in one source tar archive.

    Require exactly one top-level PKG-INFO and resolve each declared path under
    its package root, rejecting absolute paths, parent traversal and non-files.
    An archive with no License-File declarations has no entries to check here;
    this helper does not independently require the project's three licenses.
    require_abi3 additionally checks that rebuilding this archive retains the
    PyO3 abi3-py310 feature. Historical archives use the default license-only
    check. It reads without extracting or modifying the archive and prints success;
    malformed metadata, missing members and archive errors propagate.
    """
    with tarfile.open(archive) as tar:
        metadata_files = [m for m in tar.getmembers() if m.name.count("/") == 1 and m.name.endswith("/PKG-INFO")]
        if len(metadata_files) != 1:
            raise ValueError("Expected one root PKG-INFO")
        metadata = BytesParser().parsebytes(tar.extractfile(metadata_files[0]).read())
        root = metadata_files[0].name.rsplit("/", 1)[0]
        for name in metadata.get_all("License-File", []):
            path = PurePosixPath(name)
            if path.is_absolute() or ".." in path.parts or not tar.getmember(f"{root}/{name}").isfile():
                raise ValueError(f"Invalid or missing declared license file: {name}")
        if require_abi3:
            binding = tomllib.loads(tar.extractfile(f"{root}/py-rexafs/Cargo.toml").read().decode())
            features = binding["dependencies"]["pyo3"].get("features", [])
            if {feature for feature in features if feature.startswith("abi3")} != {"abi3-py310"}:
                raise ValueError("Source archive must retain exactly the abi3-py310 build feature")
    print(f"Verified source license files in {archive}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archives", nargs="+")
    parser.add_argument("--require-abi3", action="store_true")
    args = parser.parse_args()
    for archive in args.archives:
        check(archive, args.require_abi3)
