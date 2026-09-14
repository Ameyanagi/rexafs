---
title: "TypeScript · Measurement"
description: "Measurement declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.5 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Owned universal reader (unreleased). Supports text/CSV, beamline layouts,
historical binary, Athena Perl/JSON, Larix 1.0 sessions, XTUNES, gzip and HDF5. Browser callers first await
init(). No filesystem/network access, processing or input mutation occurs.
Input and expanded gzip text are each limited to 256 MiB; HDF5 numeric values
have a 256 MiB decoded budget. Gzip requires one complete member with no
trailing data. Malformed input throws Error. Inspect warnings
and choose a scan and signal; detector images require reduction/calibration.

## constructor

```typescript
constructor(data: string | Uint8Array);
```

Parse UTF-8 text or binary bytes into independent native storage.

## document

```typescript
readonly document: MeasurementDocument;
```

Copy metadata and raw arrays. Nonfinite source cells are represented by null.

## arrays

```typescript
arrays(scan?: number, mapping?: SpectrumMapping): { energy: Float64Array; mu: Float64Array };
```

Copy converted energy in eV and signal to Float64Arrays in acquisition order.
scan defaults to 0. Omit mapping only when there is exactly one detected
signal. Select a candidate's mapping or supply explicit zero-based roles.
Rejects invalid indices, conflicting roles, nonfinite selected cells,
nonpositive energy, invalid Bragg calibration and invalid intensity ratios.
Duplicates and source order remain; Spectrum construction requires strictly
increasing unique energy. No processing or cached results are changed.

## select_datasets

```typescript
select_datasets(paths: string[]): number;
```

Append copied real dataset vectors in path order; return the new scan index.
Requires at least two distinct nonempty vectors of equal length. Invalid
paths/shapes and complex arrays throw. Multidimensional detector arrays require reduction.
No processing occurs; original scans and datasets remain unchanged.

## free

```typescript
free(): void;
```

Release native data. Copied arrays remain valid; repeated free() is harmless.
