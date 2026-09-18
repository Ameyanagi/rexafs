---
title: "TypeScript · PeakFit"
description: "PeakFit declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.10.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.10/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Immutable composite XANES peak definition (since 0.2.10).
Start with new PeakFit([-20, 40]).gaussian("p1", { center: 5, area: 2, fwhm: 3 }).linear_baseline().
Defaults are Norm and E0-relative eV. Builders return NEW definitions; inputs
remain unchanged. Missing normalization runs on a copy. No smoothing or
chemical/component-count assignment is performed. Default bounds keep centers
in the interval, areas nonnegative and widths positive. Inspect termination
and warnings: local covariance is conditional on the selected model.

## constructor

```typescript
constructor(range: readonly [number, number]);
```

Create an empty Norm model over inclusive E0-relative eV; add components before fitting.

## free

```typescript
free(): void;
```

Release this definition's native memory. Do not use it afterwards. Other copies remain valid.

## flat

```typescript
flat(): PeakFit;
```

Use dimensionless flattened mu; prerequisites run on a copy. Returns a new model.

## raw_mu

```typescript
raw_mu(): PeakFit;
```

Use the original mapped absorption signal and its units. Returns a new model.

## absolute

```typescript
absolute(): PeakFit;
```

Interpret ranges, centers and baseline references as absolute eV. Returns a new model.

## reference

```typescript
reference(energy_ev: number): PeakFit;
```

Use offsets from this fixed reference energy in eV. Returns a new model.

## gaussian

```typescript
gaussian(name: string, options: PeakOptions): PeakFit;
```

Add a Gaussian: center/FWHM in eV, whole-axis area in signal units times eV. Returns a new model.

## lorentzian

```typescript
lorentzian(name: string, options: PeakOptions): PeakFit;
```

Add a Lorentzian with whole-axis area and FWHM in eV. Returns a new model.

## pseudo_voigt

```typescript
pseudo_voigt(name: string, options: PseudoVoigtOptions): PeakFit;
```

Add a common-FWHM mixture; fraction is the Lorentzian share from zero to one. Returns a new model.

## voigt

```typescript
voigt(name: string, options: VoigtOptions): PeakFit;
```

Add a true Voigt with independent Gaussian/Lorentzian FWHM in eV. Returns a new model.

## erf_step

```typescript
erf_step(name: string, options: StepOptions): PeakFit;
```

Add height*(1+erf((E-center)/scale))/2; scale is positive eV. Returns a new model.

## arctan_step

```typescript
arctan_step(name: string, options: StepOptions): PeakFit;
```

Add height*(1/2+atan((E-center)/scale)/pi); scale is positive eV. Returns a new model.

## constant_baseline

```typescript
constant_baseline(offset?: number): PeakFit;
```

Add a fitted constant named baseline, in signal units. Returns a new model.

## linear_baseline

```typescript
linear_baseline(options?: { offset?: number; slope?: number }): PeakFit;
```

Add baseline = offset+slope*E_offset; slope is signal units/eV. Returns a new model.

## exclude

```typescript
exclude(range: readonly [number, number]): PeakFit;
```

Exclude an inclusive interval in model coordinates; masked gaps are not integrated.

## parameter

```typescript
parameter(name: string, value: number, options?: PeakParameterOptions): PeakFit;
```

Replace an EXISTING parameter (e.g. p1_center or p1_width); returns a new model.
Bounds default to unbounded: supply them explicitly to retain restrictions.
An expression is a restricted tie, not executable code, and overrides vary.
Unknown names fail immediately; domains/dependencies are checked when fitting.

## as_baseline

```typescript
as_baseline(name: string): PeakFit;
```

Make a named peak part of the baseline, excluding it from the weighted center; steps cannot change role.

## solver

```typescript
solver(options?: { max_iterations?: number; tolerance?: number }): PeakFit;
```

Positive optimizer limits, default 200 iterations and 1e-10 tolerance; returns a new model.

## evaluate

```typescript
evaluate(energy: Float64Array, options?: { e0?: number }): Float64Array;
```

Evaluate at absolute energy in eV without fitting/masking. Relative models require e0; returns an owned array.

## initialize_baseline

```typescript
initialize_baseline(spectrum: Spectrum, peak_intervals: ReadonlyArray<readonly [number, number]>): PeakFit;
```

Initialize baseline-role variables outside peak intervals in model coordinates.
Returns a new starting model. Source, final masks and this definition stay unchanged.

## fit_batch

```typescript
fit_batch(spectra: readonly Spectrum[]): PeakFitOutcome[];
```

Independent unweighted fits from this starting model, one outcome per input.
A bad frame keeps an error row and does not stop later frames. Inputs are unchanged.
Runs synchronously; use a Web Worker for large browser batches.

## to_json

```typescript
to_json(): string;
```

Serialize the complete initial definition, including constraints and masks.

## from_json

```typescript
static from_json(json: string): PeakFit;
```

Restore and validate a complete definition. Browser init must have completed.
