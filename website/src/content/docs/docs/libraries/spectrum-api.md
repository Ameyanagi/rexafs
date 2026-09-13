---
title: "The spectrum workflow"
description: "Stages, outputs, units and invalidation across the libraries."
audience: user
---

All interfaces use the same mutable spectrum workflow. A requested stage computes
missing prerequisites. Explicitly requesting a stage recomputes that stage.

| Stage | Calculation | Results |
|---|---|---|
| `find_e0()` | Estimate the edge from the absorption derivative | `e0()` in eV |
| `normalize()` | Subtract a pre-edge baseline and divide by the edge step | `norm()`, `flat()`, `pre_edge()`, `post_edge()` |
| `calc_background()` | Estimate a smooth background with AUTOBK | `k()` in Å⁻¹, dimensionless `chi()` |
| `fft()` | Apply k weighting/window and transform | `r()` in Å, `chir_mag()`, `chir_real()`, `chir_imag()` |
| `ifft()` | Filter in R and transform to q | `q()` in Å⁻¹, filtered `chiq()` |

The dimensions of Fourier values depend on k weighting. See
[the exact transform convention](/docs/science/processing/) before labeling axes.
Use `kwin_k()` as the matching axis for `kwin()`; a Larch FFT grid may differ
from the background grid returned by `k()`.

## Inputs and errors

Supply two finite, equal-length arrays with strictly increasing energy. Energy is
in eV, not keV; μ is already constructed absorption. Invalid data, unsupported
algorithms and failed numerical solves return errors. Python raises exceptions,
TypeScript throws, and Rust returns `Result`.

Before computation, result getters can be unavailable: `None` in Python,
`undefined` in TypeScript, or `None` in a Rust `Option`. Python/TypeScript results
are independent array copies. Rust `k()` and `chi()` borrow slices, and `chir()`
borrows the complete complex FFT representation; its other array getters return
owned vectors. No getter runs a calculation implicitly.

## Settings and dependent results

- Replacing data clears the edge estimate and derived results.
- Changing normalization invalidates background and both transforms.
- Changing background invalidates both transforms.
- Changing forward-transform settings invalidates forward and inverse results.
- Python and TypeScript settings are copied when assigned; editing the original
  object alone does not change the spectrum.
- Rust setters take ownership. Use `.clone()` to keep a reusable configuration.

Prefer setters to direct Rust field edits; legacy field edits require explicit
invalidation. Existing project defaults and automatic settings should be recorded
when reproducing a result in another interface.

Stable 0.2.4 uses zero-argument settings constructors in Python/TypeScript, with
fields assigned afterward. See [Python](/docs/libraries/python/) and
[TypeScript](/docs/libraries/typescript/) for complete examples. New constructor
arguments and inverse setters belong to the [Next API](/docs/reference/).
