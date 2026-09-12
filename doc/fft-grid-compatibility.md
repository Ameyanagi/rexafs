# AUTOBK and FFT compatibility in 0.2.4

Issue [#20](https://github.com/Ameyanagi/rexafs/issues/20) separated differences in
background removal from differences in the public output transform. The
[fixed-λ AUTOBK change](autobk-fixed-penalty.md) previously resolved the default
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
derivatives of the [implemented legacy objective](../crates/rexafs/src/xafs/background.rs),
not an additional physical model. For normalization and Fourier conventions,
see [processing theory](processing-theory.md).

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

## Independent validation

[`generate-fft-grid-reference.py`](../scripts/generate-fft-grid-reference.py)
loads only `ftwindow` and `xftf_prep` from the immutable
[XrayLarch source](https://github.com/xraypy/xraylarch/blob/860d8a690c81eefb0e61dee4ca3703ef4b67e93d/larch/xafs/xafsft.py).
The retained JSON records that source's SHA-256 and NumPy/SciPy versions. Its 42
cases cover seven windows, endpoints below/at/above kmax, asymmetric tapering,
and uniform/nonuniform input including nonzero starting k. Expected transforms
use NumPy's FFT; they are never regenerated by tests.

Both Rust backends compare every window value within `2e-12` and every complex
FFT component within `2e-10`. A fixed-distance shell fit uses the complex R-space
samples between 1 and 3 Å, with an off-shell contaminating signal. Fitted
amplitude and phase agree with NumPy least squares within `1e-10`. Separate tests
cover invalid inputs, allocation bounds, serialization defaults and window axes
at different sampling steps. Python and Node tests exercise the explicit choice,
cache invalidation, unchanged background χ, and restoration of Input mode.

The [measured Cu/Ni/Ru comparison](validation/2026-09-10-numerical-compat/README.md)
retains complete before/after arrays and input hashes. Fixed-λ default arrays are
exactly unchanged on this host; legacy iterative changes are quantified separately.
