---
title: "Python · PeakFitOutcome"
description: "PeakFitOutcome signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

One independent batch outcome in input order (since 0.2.10).
Exactly one of result/error is present. Nonconvergence is retained as a result.

## index

```python
index(self) -> int
```

Zero-based input index, also retained on failure.

## result

```python
result(self) -> PeakFitResult | None
```

Owned numerical result; inspect its termination and warnings.

## error

```python
error(self) -> str | None
```

Failure reason, otherwise None.
