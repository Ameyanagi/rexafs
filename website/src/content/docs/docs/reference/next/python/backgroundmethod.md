---
title: "Python · BackgroundMethod"
description: "BackgroundMethod signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


Rust algorithm selection. Prefer passing settings directly to the spectrum setter.


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)



## AUTOBK


```python
AUTOBK(parameters: AUTOBK) -> BackgroundMethod
```

Copy AUTOBK settings into a background method.


## new_autobk


```python
new_autobk() -> BackgroundMethod
```

Create the recommended default AUTOBK method.


## new_ilpbkg


```python
new_ilpbkg() -> BackgroundMethod
```

Create an unimplemented ILPBkg placeholder; processing raises RuntimeError.
