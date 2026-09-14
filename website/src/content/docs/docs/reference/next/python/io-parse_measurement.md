---
title: "Python · io.parse_measurement"
description: "io.parse_measurement signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.6 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

## parse_measurement

```python
parse_measurement(data: bytes | str) -> Measurement
```

Read text or binary measurement content without filesystem/network access.

Universal Rust reader, added in 0.2.6: XDI, beamline text, CSV, historical binary,
Athena (Perl/JSON, optionally gzip), Larix 1.0 sessions, XTUNES and HDF5. Strings are encoded as UTF-8.
Returns owned scans, original metadata, signal choices and saved arrays;
inspect .document and select .arrays() or .spectrum() to convert a scan.
Ambiguous detector roles and image-only datasets require explicit selection
or reduction; reading them does not imply a ready-to-process spectrum.
Input and expanded gzip sizes are limited to 256 MiB each. Gzip requires one
complete member with no trailing data. Invalid containers
and malformed rows raise ValueError. No input data or settings are modified.
