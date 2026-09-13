---
title: "Python · type aliases"
description: "Literal types and module attributes."
audience: user
pagefind: false
---

**next API**. See the [version guide](/docs/reference/).

```python
__version__: str

FFTGrid: TypeAlias = Literal['Input', 'Larch']

FTWindow: TypeAlias = Literal['Hanning', 'Parzen', 'Welch', 'Gaussian', 'Sine', 'KaiserBessel', 'FHanning']

AUTOBKSolver: TypeAlias = Literal['TrustRegionDogLeg', 'LegacyLm', 'LinearDirect']

AUTOBKClampScalePolicy: TypeAlias = Literal['FixedPenalty', 'Fixed', 'TwoPass']
```
