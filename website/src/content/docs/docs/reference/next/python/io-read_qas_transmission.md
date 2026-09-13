---
title: "Python · io.read_qas_transmission"
description: "io.read_qas_transmission signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


Read a QAS transmission scan as energy (eV) and mu = ln(I0 / It).

I0 and It are positive incident/transmitted intensities in matching units.
ln is the natural logarithm, so mu is dimensionless optical depth.
Accepts a filename or pathlib.Path. Returns an unprocessed Spectrum;
call .fft() to run the default pipeline. File and parse failures raise
RuntimeError. Uses native QAS column conventions.


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)



## read_qas_transmission


```python
read_qas_transmission(path: str | PathLike[str]) -> Spectrum
```

Read a QAS transmission scan as energy (eV) and mu = ln(I0 / It).

I0 and It are positive incident/transmitted intensities in matching units.
ln is the natural logarithm, so mu is dimensionless optical depth.
Accepts a filename or pathlib.Path. Returns an unprocessed Spectrum;
call .fft() to run the default pipeline. File and parse failures raise
RuntimeError. Uses native QAS column conventions.
