---
title: "Python · io.SignalConversion"
description: "io.SignalConversion signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Names or zero-based indices for direct, transmission or detector/monitor ratio roles.

## kind

```python
kind: Literal['direct', 'transmission', 'ratio']
```

Arithmetic: direct, transmission or ratio. Required at runtime.

## column

```python
column: ColumnSelector
```

Exact name or zero-based stored-signal column for direct.

## incident

```python
incident: ColumnSelector
```

Exact name or zero-based incident monitor for transmission or ratio.

## transmitted

```python
transmitted: ColumnSelector
```

Exact name or zero-based transmitted intensity for transmission.

## detectors

```python
detectors: list[ColumnSelector]
```

Nonempty list of distinct detector names or indices for ratio.
