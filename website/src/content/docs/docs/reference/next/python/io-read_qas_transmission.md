---
title: "Python · io.read_qas_transmission"
description: "io.read_qas_transmission signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout, including additions not available in rexafs 0.2.4.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

## read_qas_transmission

```python
read_qas_transmission(path: str | PathLike[str]) -> Spectrum
```

Read a whitespace-delimited QAS transmission scan into a new Spectrum.

The first three columns are energy in eV, incident intensity I0 and
transmitted intensity It; additional columns are ignored and # starts
a comment. The reader computes mu = ln(I0 / It), where ln is the natural
logarithm and the intensities have matching units. Positive intensities
give dimensionless optical depth, without dividing by sample thickness.
This reader is unsuitable for files that already contain absorption mu.

path accepts a filename string or pathlib.Path. The returned spectrum
owns its data and is unprocessed: call .fft() to run the default pipeline.
Missing files, unreadable data or fewer than three columns raise
RuntimeError. The reader sorts energy and its calculated mu together
into increasing order when needed, but retains duplicate energy rows.
It does not enforce positive intensities or finite ratios: check both
measured intensities before taking the ratio. Later processing rejects
non-finite data; duplicate energies may require cleanup for the selected
numerical stage. This permissive reader differs from the strict array
constructor, which rejects unordered or duplicate energy values.

For the transmission equation and its physical assumptions, see
[Newville, Fundamentals of XAFS, section 4](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf).
