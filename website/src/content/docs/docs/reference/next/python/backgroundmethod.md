---
title: "Python · BackgroundMethod"
description: "BackgroundMethod signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout, including additions not available in rexafs 0.2.4.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Select a background algorithm and own a copy of its settings.

Use BackgroundMethod.AUTOBK(parameters) to configure the spline background
or new_autobk() for the recommended defaults, then pass the result to
Spectrum.set_background_method(). Creating a method does not process
data. ILPBkg is a named placeholder and is not implemented.

## AUTOBK

```python
AUTOBK(parameters: AUTOBK) -> BackgroundMethod
```

Copy AUTOBK parameters into a background method.

Assign the result with spectrum.set_background_method(method), then call
calc_background() or a later stage. Edits to the original parameters do
not change an already created method or spectrum.

## new_autobk

```python
new_autobk() -> BackgroundMethod
```

Create a background method using the recommended AUTOBK defaults.

This selects rbkg=1.0 angstrom, LinearDirect and FixedPenalty with
clamp_lambda=0.001. Assign it to a spectrum before processing; no
background is fitted by this factory.

## new_ilpbkg

```python
new_ilpbkg() -> BackgroundMethod
```

Create the unimplemented ILPBkg background placeholder.

Selecting it preserves the requested algorithm, but calc_background()
and dependent stages raise RuntimeError rather than substitute AUTOBK.
Use new_autobk() for the implemented background workflow.
