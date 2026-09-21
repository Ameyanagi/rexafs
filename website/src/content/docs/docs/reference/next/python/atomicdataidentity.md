---
title: "Python · AtomicDataIdentity"
description: "AtomicDataIdentity signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.13 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Exact offline provider, database version and decoded-data checksum.

## provider

```python
provider: str
```

Named provider/interpolation implementation version.

## data_version

```python
data_version: str
```

Upstream database version.

## data_sha256

```python
data_sha256: str
```

SHA-256 of the actual decoded data.
