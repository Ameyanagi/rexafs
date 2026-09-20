---
title: "Python · MbackErfc"
description: "MbackErfc signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Optional smooth fluorescence background for MBACK (since 0.2.10).

The line must originate at the selected absorber edge. width=(low, high)
gives positive eV bounds; amplitude=(low, high) gives finite f2-unit bounds.
family=False selects one exact line such as Ka1; True selects a within-shell
family such as Ka. This is not an over-absorption correction. Settings are copied.

## MbackErfc

```python
MbackErfc(line: str, *, width: tuple[float, float], amplitude: tuple[float, float], family: bool=False)
```

Select an emission and explicit increasing width/amplitude bounds.
