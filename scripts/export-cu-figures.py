#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# dependencies = ["cairosvg==2.8.2"]
# ///
"""Finish the ruviz tutorial figures as vector PDFs and compact 600 dpi PNGs.

Run with ``uv run --no-project scripts/export-cu-figures.py <figure-directory>``
after ``cu_reduction_figures``. Cairo must be installed on the system. On macOS
with Homebrew Cairo, set DYLD_FALLBACK_LIBRARY_PATH to its lib directory if needed.
The PNG operation recompresses the existing lossless stream; it preserves every
pixel, filter byte and metadata chunk, including physical resolution. No spectra
are read, fitted, interpolated or regenerated here. Existing exports are replaced.
"""

import argparse
from pathlib import Path
import struct
import zlib

import cairosvg


def compact_png(path: Path) -> None:
    """Losslessly recompress IDAT chunks while preserving all other PNG bytes."""
    original = path.read_bytes()
    if original[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"Not a PNG: {path}")
    chunks = []
    offset = 8
    while offset < len(original):
        size = struct.unpack_from(">I", original, offset)[0]
        end = offset + size + 12
        if end > len(original):
            raise ValueError(f"Truncated PNG: {path}")
        chunks.append(original[offset:end])
        offset = end
    payload = b"".join(chunk[8:-4] for chunk in chunks if chunk[4:8] == b"IDAT")
    compressed = zlib.compress(zlib.decompress(payload), level=9)
    idat = b"IDAT" + compressed
    replacement = struct.pack(">I", len(compressed)) + idat + struct.pack(">I", zlib.crc32(idat))
    result = bytearray(original[:8])
    inserted = False
    for chunk in chunks:
        if chunk[4:8] != b"IDAT":
            result.extend(chunk)
        elif not inserted:
            result.extend(replacement)
            inserted = True
    if len(result) < len(original):
        path.write_bytes(result)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    for name in ("recipe", "pca", "comparison"):
        svg = args.directory / f"{name}.svg"
        png = args.directory / f"{name}.png"
        if not svg.is_file() or not png.is_file():
            parser.error(f"Render {name}.svg and {name}.png with ruviz first.")
        cairosvg.svg2pdf(url=str(svg), write_to=str(args.directory / f"{name}.pdf"))
        compact_png(png)
        print(f"{name}: vector PDF; lossless PNG {png.stat().st_size:,} bytes")


if __name__ == "__main__":
    main()
