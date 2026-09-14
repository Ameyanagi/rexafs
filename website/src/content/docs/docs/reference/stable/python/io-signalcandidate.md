---
title: "Python · io.SignalCandidate"
description: "io.SignalCandidate signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.6.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.6/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Header-supported signal label and conversion; multiple choices require selection.

## name

```python
name: str
```

Human-readable signal label.

## mapping

```python
mapping: SpectrumMapping
```

Complete conversion supported by source metadata.
