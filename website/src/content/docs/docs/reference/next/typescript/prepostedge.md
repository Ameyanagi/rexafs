---
title: "TypeScript · PrePostEdge"
description: "PrePostEdge declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.7 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Settings for absorption-edge normalization and flattening.

The pre-edge fit estimates the smooth baseline before the edge. The post-edge fit estimates
the edge step and the trend used by flat(). All energy bounds are offsets from E0 in eV, not
absolute energies. Undefined fields select data-dependent values where described; inspect the
measured fit regions before interpreting a normalized spectrum.

Settings are copied when assigned to a spectrum. Changing this object afterwards
requires assigning it again; free() releases only this object's Wasm allocation.

Background and normalization conventions are explained in [Newville, Fundamentals of
XAFS](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf). Automatic range and
polynomial rules are rexafs implementation choices in
[PrePostEdge](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/normalization.rs).

## constructor

```typescript
constructor(options?: PrePostEdgeOptions);
```

Create owned settings with the recommended defaults described below. Browser callers must
await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
and free() this object when finished.

Automatic fields are resolved on the spectrum's copy during processing; resolved values
are not written back into the original settings object. Resolved values stay in the
spectrum's settings until those settings are replaced.

Named options were added in 0.2.5. Version 0.2.4 uses this constructor without
arguments, followed by property assignment.

## free

```typescript
free(): void;
```

Release this object's Wasm allocation. Do not call methods, read fields or free it again
afterwards. Arrays and settings already copied elsewhere remain valid.

## pre_edge_start

```typescript
pre_edge_start: number | undefined;
```

Lower pre-edge fit bound, as an energy offset from E0 in eV. Default: undefined selects a
rounded estimate near the beginning of the measured range. Choose a region below the
absorption edge without other edges or glitches; this region determines the subtracted
baseline.

## pre_edge_end

```typescript
pre_edge_end: number | undefined;
```

Upper pre-edge fit bound relative to E0, in eV. Default: undefined derives a rounded value
from pre_edge_start. The pre-edge region is normally below E0 (negative offsets); moving it
toward the edge can include near-edge structure in the baseline fit.

## norm_start

```typescript
norm_start: number | undefined;
```

Lower post-edge fit bound relative to E0, in eV. Default: undefined derives it from
norm_end, normally capped at 25 eV and kept at least 10 eV below norm_end. This region
estimates the absorption edge step and the trend removed by flat().

## norm_end

```typescript
norm_end: number | undefined;
```

Upper post-edge fit bound relative to E0, in eV. Default: undefined rounds the available
post-edge extent to a nearby 5 eV boundary, without extending beyond the measured range.
Choose the range before a second absorption edge if one is present.

## norm_polyorder

```typescript
norm_polyorder: number | undefined;
```

Integer degree of the polynomial fitted to pre-edge-subtracted absorption above E0.
Default: undefined selects 0, 1 or 2 for post-edge fit spans below 50, below 350, or at
least 350 eV. Values are clamped to 0 through 5. Higher order follows more curvature but
can absorb spectral structure.

## n_victoreen

```typescript
n_victoreen: number | undefined;
```

Integer energy exponent used to model the pre-edge baseline. Default: undefined resolves to
0, a straight-line fit to mu(E). For exponent n, the code fits a line to mu(E) * E^n and
divides the fitted line by E^n; E is energy in eV. Use 0 unless this additional energy
dependence is justified.

## e0

```typescript
e0: number | undefined;
```

Absorption-edge energy in eV. Default: undefined uses the spectrum E0 if assigned,
otherwise derivative-based edge detection. E0 sets the origin of the fit offsets and the
subsequent energy-to-k conversion. A supplied value must be finite and strictly between
the measured energy endpoints; spectrum processing throws an Error otherwise.

## edge_step

```typescript
edge_step: number | undefined;
```

Absorption step used as the normalization divisor, in the same units as mu. Default:
undefined estimates post_edge - pre_edge at the input sample nearest E0. Use a positive
physical step; the implementation floors finite values below 1e-12 to 1e-12 and rejects
nonfinite steps. Changing it rescales norm() and chi().
