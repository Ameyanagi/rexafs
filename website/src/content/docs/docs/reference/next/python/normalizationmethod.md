---
title: "Python · NormalizationMethod"
description: "NormalizationMethod signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


Rust algorithm selection. Prefer passing settings directly to the spectrum setter.


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)



## PrePostEdge


```python
PrePostEdge(parameters: PrePostEdge) -> NormalizationMethod
```

Copy pre/post-edge settings into a normalization method.


## new_prepostedge


```python
new_prepostedge() -> NormalizationMethod
```

Create automatic pre/post-edge normalization settings.


## new_mback


```python
new_mback() -> NormalizationMethod
```

Create an unimplemented MBack placeholder; processing raises ValueError.
