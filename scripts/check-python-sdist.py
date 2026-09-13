"""Check that every declared sdist license file is present at its metadata path."""
from email.parser import BytesParser
from pathlib import PurePosixPath
import sys
import tarfile


def check(archive):
    """Verify the existing License-File declarations in one source tar archive.

    Require exactly one top-level PKG-INFO and resolve each declared path under
    its package root, rejecting absolute paths, parent traversal and non-files.
    An archive with no License-File declarations has no entries to check here;
    this helper does not independently require the project's three licenses.
    It reads without extracting or modifying the archive and prints success;
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
    print(f"Verified source license files in {archive}")


if __name__ == "__main__":
    for archive in sys.argv[1:]:
        check(archive)
