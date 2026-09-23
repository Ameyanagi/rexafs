---
title: "Python · AtomicReference"
description: "AtomicReference signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Exact atomic dataset and numerical table identity.

## data

```python
data: AtomicDataIdentity
```

Actual loaded dataset identity.

## table

```python
table: Literal['ChantlerF2LogLogV1', 'ElamTotalV1', 'ElamTransitionsV1']
```

Table and interpolation/contribution profile.
