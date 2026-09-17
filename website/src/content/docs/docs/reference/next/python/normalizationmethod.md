---
title: "Python · NormalizationMethod"
description: "NormalizationMethod signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Select the normalization algorithm and own a copy of its settings.

Use NormalizationMethod.PrePostEdge(parameters) for configured pre/post-edge
normalization or new_prepostedge() for automatic settings, then pass the
result to Spectrum.set_normalization_method(). Creating a method does not
process data. The no-argument MBack selector has no absorber/edge and cannot
normalize. Unreleased: pass configured MBack settings directly to Spectrum.

## PrePostEdge

```python
PrePostEdge(parameters: PrePostEdge) -> NormalizationMethod
```

Copy PrePostEdge parameters into a normalization method.

Assign the result with spectrum.set_normalization_method(method), then
call normalize() or a later stage. The original parameters remain
independent: edits do not change an already created method or spectrum.

## new_prepostedge

```python
new_prepostedge() -> NormalizationMethod
```

Create a normalization method with automatic pre/post-edge settings.

E0, fitting ranges, polynomial degree and edge step are inferred when
processing runs. Equivalent to NormalizationMethod.PrePostEdge(PrePostEdge());
assign the method to a spectrum to use it. No data are processed here.

## new_mback

```python
new_mback() -> NormalizationMethod
```

Create the historical empty MBack normalization selector.

Selecting it preserves the requested algorithm, but normalize() and
dependent stages raise ValueError rather than substitute another method.
Unreleased: use MBack(element, edge) for full MBACK. Through 0.2.9 the
MBACK algorithm was unimplemented. This no-argument selector still lacks identity.
