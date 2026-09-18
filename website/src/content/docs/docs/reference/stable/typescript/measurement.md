---
title: "TypeScript · Measurement"
description: "Measurement declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.10.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.10/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Owned universal reader (since 0.2.6). Supports text/CSV, beamline layouts,
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

Select columns with exact names or indices, for example arrays({energy:"energy", i0:"I0", it:"It"}).
Missing/duplicate names fail. Retains the same owned arrays, ordering, conversion checks and
no-processing behavior as the positional overload. Cannot combine options with a mapping.

## arrays

```typescript
arrays(options: MeasurementOptions): { energy: Float64Array; mu: Float64Array };
```

Select columns with exact names or indices, for example arrays({energy:"energy", i0:"I0", it:"It"}).
Missing/duplicate names fail. Retains the same owned arrays, ordering, conversion checks and
no-processing behavior as the positional overload. Cannot combine options with a mapping.

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
