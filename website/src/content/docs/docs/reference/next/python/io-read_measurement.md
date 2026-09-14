---
title: "Python · io.read_measurement"
description: "io.read_measurement signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.6 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

## read_measurement

```python
read_measurement(path: str | PathLike[str]) -> Measurement
```

Read a local measurement file through the universal reader added in 0.2.6.

Accepts a filename or pathlib.Path; content determines the format, not the
extension. For an unambiguous first scan, use read_measurement(path).arrays()
for energy in eV and signal, or .spectrum() for an unprocessed Spectrum.
Returns an owned Measurement; inspect .document, then select a
scan and mapping with .arrays() or .spectrum(). No processing, file writes
or network requests occur. Input and expanded gzip are limited to 256 MiB
each. File errors raise OSError, and parsing/conversion errors raise
ValueError. The document records ambiguous channels and HDF5 limitations.
