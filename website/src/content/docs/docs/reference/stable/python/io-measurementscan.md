---
title: "Python · io.MeasurementScan"
description: "io.MeasurementScan signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.9.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

One original scan with headers, metadata, columns, signal choices and diagnostics.

## id

```python
id: str
```

Source record identifier; duplicate display names retain distinct identifiers.

## label

```python
label: str
```

Display label, without inferred scientific meaning.

## columns

```python
columns: list[MeasurementColumn]
```

Original channel order; columns have equal lengths.

## header

```python
header: str
```

Original text header; XTUNES retains the complete record text.

## metadata

```python
metadata: dict[str, str]
```

Extracted source metadata; XTUNES ordered_parameters contains ordered JSON triples of section, key and value.

## signals

```python
signals: list[SignalCandidate]
```

Suggested mappings; zero or multiple choices require explicit selection.

## warnings

```python
warnings: list[str]
```

Unit assumptions, channel ambiguity and format limitations.
