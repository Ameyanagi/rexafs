---
title: "Python · AtomicReference"
description: "AtomicReference signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.11 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

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
