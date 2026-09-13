---
title: "Fourier and Larch compatibility"
description: "Compare sampling conventions and legacy AUTOBK behavior."
audience: user
---

Issue [#20](https://github.com/Ameyanagi/rexafs/issues/20) separated differences in
background removal from differences in the public output transform. The
[fixed-λ AUTOBK change](/docs/science/autobk/) previously resolved the default
background objective. Version 0.2.4 addresses the remaining legacy derivative
and FFT grid questions.

## Legacy iterative AUTOBK

The historical clamp residual is `weight × s(c) × chi(c)`, where
`s(c) = 1 + 100 × mean(h(c)²)` and `h` contains the real/imaginary low-R residual.
Its derivative is now

```text
dresidual/dc = weight × (s × dchi/dc + chi × ds/dc)
ds/dc = 200 × dot(h, dh/dc) / len(h)
```

Here `c` denotes a spline coefficient (apply the expression to every coefficient
to build the Jacobian), `chi(c)` is an endpoint residual, `weight` is its fixed
endpoint multiplier, and `len(h)` counts the real/imaginary low-R entries.
The second equation follows by differentiating the sum of squared entries of h;
the first follows from the product rule. Omitting `chi × ds/dc` differentiates
a frozen scale instead of the scale actually used by the residual. These are
derivatives of the [implemented legacy objective](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/background.rs),
not an additional physical model. For normalization and Fourier conventions,
see [processing theory](/docs/science/processing/).

Both array backends include the previously omitted second term. The FFT head
and spectrum used by that term are computed once per Jacobian. The frozen-scale
linear design matrix still uses zero scale derivative. Residual weights, scale,
slices (including the legacy high-end exclusion of the last sample), and default
fixed-λ objective are unchanged. A failed/nonfinite legacy LM solve now returns
an error instead of returning its last coefficients as a successful result.

Central differences cover 64 combinations of clamp length, endpoint weights,
empty/nonempty FFT heads and dynamic/frozen scale in each backend, including an
optional χ standard. Both cached and uncached spline-basis paths are exercised.
Relative Jacobian error must be below `1e-7`.

The regression spectrum also compares LM with a numerical Jacobian. Both
converge in 45 residual evaluations to objective `3.299013380277`, with χ relative
L2 difference below `1e-6`. Repeating the old omitted-term derivative takes 48
evaluations and stops at objective `3.767250202953`; its χ differs by 11.22%.
These are synthetic diagnostic results, not a claim of measured-data accuracy.

Recomputing a project that explicitly uses `LegacyLm`, legacy dog-leg, or a
fallback to them can therefore change its result. Keep the previous release
when reproducing that earlier numerical behavior. The current fixed-λ default
and legacy frozen-scale direct solves do not use this dynamic Jacobian.

## Backend scope

The default nalgebra backend implements the recommended FixedPenalty AUTOBK
objective. The optional Rust `ndarray-compat` feature selects a historical
backend with only Fixed/TwoPass clamp policies and a rounded, rather than
floored, automatic spline coefficient count. It does not expose `clamp_lambda`.
Matching the public FFT grid does not remove these background differences.
Desktop, Python and Wasm use the default backend.

Both public forward FFT implementations use the actual `kstep / sqrt(pi)`
amplitude factor after an unnormalized negative-exponent FFT. FixedPenalty
AUTOBK instead uses the fixed internal reference multiplier `0.05 / sqrt(pi)`;
its selected kstep still controls the R axis. Legacy AUTOBK policies use the
actual kstep for both scale and R axis. These are separate objective conventions.

## Explicit output FFT grid

The default Rust backend, desktop, Python and JavaScript retain **Input grid**.
It weights and windows the supplied k samples without resampling. Projects and
serialized FFT settings that omit the new field retain this behavior.

Choose **Transform → Advanced → sampling grid → Larch grid** for the extended
window domain used by XrayLarch. Equivalent API settings are:

```python
ft = rexafs.XrayFFTF()
ft.grid = "Larch"  # "Input" restores the historical default
spectrum.set_fft(ft).fft()
k_for_window, window = spectrum.kwin_k(), spectrum.kwin()
```

```javascript
const ft = new XrayFFTF();
ft.grid = "Larch";
spectrum.set_fft(ft).fft();
```

```rust
let ft = rexafs::XrayFFTF {
    grid: rexafs::FFTGrid::Larch,
    ..Default::default()
};
spectrum.set_fft(ft).fft()?;
```

Larch mode linearly resamples χ from k=0 at the selected FFT step, holding the
nearest endpoint value outside the supplied range. It constructs the window
through `max(last measured k, kmax + dk2)` using Larch's integer grid counts,
then truncates the weighted spectrum and window to the measured range before
zero-padding. It requires finite, increasing, nonnegative k and enough NFFT
points for the extended window. The background's k/χ arrays remain unchanged.
Use `kwin_k()` for the window axis when FFT and background steps differ; desktop
plots and thumbnails do this automatically.

The optional `ndarray-compat` backend already constructed an extended grid and
retains that default. Its output transform now applies the window it previously
calculated but accidentally omitted from the FFT multiplication. This bug fix
changes ndarray output transforms. Select `FFTGrid::Input` explicitly when
comparing its input-domain behavior with the default backend.

The choice is saved, participates in parameter copying, overrides, undo and cache
invalidation, and does not alter AUTOBK's internal transform or FEFF fitting's
separate transform model. Compatibility does not imply equivalence of different
background objectives or general FEFF fit results. rexafs continues to expose
the real FFT's Nyquist bin; Larch's public output convention omits that final bin.
