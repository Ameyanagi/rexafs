---
title: "Python · io.SignalConversion"
description: "io.SignalConversion signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.5 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Zero-based roles: direct column, transmission incident/transmitted, or ratio detectors/incident.

## kind

```python
kind: Literal['direct', 'transmission', 'ratio']
```

Arithmetic: direct, transmission or ratio. Required at runtime.

## column

```python
column: int
```

Zero-based stored-signal column for direct.

## incident

```python
incident: int
```

Zero-based incident monitor for transmission or ratio.

## transmitted

```python
transmitted: int
```

Zero-based transmitted intensity for transmission.

## detectors

```python
detectors: list[int]
```

Nonempty, unique zero-based detector columns for ratio.
