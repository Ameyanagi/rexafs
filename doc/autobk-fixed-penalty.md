# Fixed-λ AUTOBK

AUTOBK estimates a smooth absorption background by suppressing low-distance
Fourier components of the remaining EXAFS signal. The original method is
described by [Newville et al., *Physical Review B* 47, 14126–14131 (1993)](https://doi.org/10.1103/PhysRevB.47.14126).
The fixed endpoint penalty below is a **rexafs-specific modification**, not an
objective attributed to that paper. Start with the
[processing theory guide](processing-theory.md) for the absorption, k and R definitions.

New analyses use `LinearDirect` with `clamp_scale_policy = FixedPenalty` and
`clamp_lambda = 0.001`. This implements the weak fixed penalty tested in the
[clamp study](benchmarks/2026-09-07-clamp-study/README.md). The study's training
minimum was λ=0, with weak penalties practically tied; 0.001 is a conservative
weak-penalty choice, not a universally optimal value.

For the vector of fitted cubic-spline coefficients **c**, let χ(c) be the edge-step-normalized spectrum,
minus an optional normalized χ standard **for the fitting objective only**.
Let **h(c)** contain the real and imaginary low-R Fourier residuals. The spline is linear in **c**. Its normalized residual χ(c) is therefore an
affine function of **c** when the energy grid, edge step, window and other
settings are fixed. The low-R residual vector **h(c)** contains the transformed
mismatch that should be small if the background has been removed. The objective is

```text
J(c) = ||h(c)||² / m
       + λ × [w_lo² Σ χ(first n points)² + w_hi² Σ χ(last n points)²] / N_active

m        = number of real/imaginary low-R residual entries
n        = min(nclamp, number of χ(k) points)
N_active = n × (number of nonzero endpoint weights)
```

Here, `||h||²` is the sum of squared entries, and each endpoint sum uses the
corresponding elements of χ(c). λ is `clamp_lambda`; `w_lo` and `w_hi` are the
absolute endpoint weights. The division by `m` makes the first term a mean
squared residual, and the division by `N_active` averages over enabled endpoint
samples. These are numerical residual conventions with the Fourier amplitude
scale specified below; λ is not a physical material property.

A zero weight excludes that end from both the sum and its denominator. If both
weights are zero, `nclamp = 0`, or `clamp_lambda = 0`, the endpoint term is absent.
The weights are the absolute values of `clamp_lo` and `clamp_hi`.

Defaults are `nclamp = 3`, `clamp_lo = 0`, and `clamp_hi = 1`: no low-end penalty,
and λ times the mean χ² of the final three points, including the last point.
At kstep=0.05 and kmax=12 these are k=11.90, 11.95, and 12.00 Å⁻¹. If enabled,
the first three points are k=0, 0.05, and 0.10 Å⁻¹. These are endpoints of the
output k grid; kmin controls the transform window and spline domain.

Doubling λ doubles the endpoint contribution to the objective. Doubling an
endpoint weight multiplies that end's contribution by four. Enabling both ends
shares the total penalty across twice as many endpoint points. There is no
initial-residual scale, update during a fit, second pass, or nonlinear fallback.

The low-R transform uses the k-weight and window selected for AUTOBK, with the
reference FFT amplitude factor `0.05 / sqrt(pi)` used by Larch's AUTOBK residual.
The selected kstep and nfft determine the physical R grid and cutoff. The fixed
reference amplitude also makes the reported λ directly comparable with the
prototype at both tested k steps; changing the window or k-weight changes the
objective and can change the effective balance with the endpoint term.

## Implementation

Energy-to-k conversion uses the CODATA 2022 electron mass, `9.1093837139e-31 kg`,
with the exact SI Planck constant and elementary charge. This gives
`E - E0 = 3.809982110968585 × k²`, where E is photon energy in eV,
E0 is the edge energy in eV, and k is photoelectron wave number in Å⁻¹.
This matches the current SciPy/Larch reference. Both Rust array backends and Athena export share the same constants.
See [NIST CODATA 2022](https://physics.nist.gov/cuu/pdf/wall_2022.pdf).

The coefficient count uses `1 + floor(2 rbkg (kmax-kmin) / pi)`, bounded to 5–128,
with optional explicit nknots. The low-R cutoff uses floor. Cubic not-a-knot
interpolation and polynomial extrapolation reproduce the Larch model used in
the study. Raw-data interpolation uses O(n) memory/time rather than a dense
n-by-n solve. When possible, resampling the spline basis is eliminated using
the fact that its interior knots are a subset of the raw interpolant's knots;
otherwise each basis column is resampled explicitly in O(n).

For fixed settings, write χ(c) = y − Bc, where y is the normalized data
minus any objective-only standard and each column of B is one normalized spline
basis function on the output grid. Let T apply the weighted low-R transform and
let A select endpoint samples and multiply them by their endpoint weights.
Multiplying J(c) by the positive constant m does not change its minimizer, giving

```text
minimize || T(y − Bc) ||² + || sqrt(λ m / N_active) A(y − Bc) ||²
```

This explains the endpoint row multiplier `sqrt(λ m / N_active)`. When the
endpoint term is disabled, those rows are omitted rather than dividing by zero.
Stacking the transform and endpoint rows makes one linear least-squares system.
Singular-value decomposition (SVD) separates its independent coefficient
directions from poorly determined ones. Column scaling improves numerical
conditioning without changing the mathematical objective. The implementation
is in [fixed.rs](../crates/rexafs/src/xafs/background/fixed.rs). Rank, conditioning, finite values, and stationarity are checked. A failure
returns an error; it never changes λ or inserts regularization to force a result.
Compatible spline/FFT geometry, the scaled design matrix and its SVD factors
are cached together after exact geometry comparison. Each spectrum supplies a
new right-hand side and obtains new coefficients; the cache does not reuse a
previous spectrum's answer. The condition limit is checked on each solve.
The solution is invariant to consistent scaling of μ and the edge step.

Regression checks compare every χ point with the independent SciPy solution of
the same fixed objective using `abs(error) <= 1e-12 + 1e-11 * abs(reference)`.
The absolute term handles zero crossings. Rust covers cache use and absorption
gains as well as λ=0/0.001/1; the Python and Wasm bindings use the same eight
measured/synthetic fixtures and three λ values. The full benchmark validates
966 archived fits plus 30 endpoint/kmin cases with this bound.

Stock Larch uses a different, dynamic clamp objective. Its separate compatibility
gate requires relative L2 χ error at most `5e-5` (**0.005%**) over 2≤k≤kmax for
Cu/Ni/Ru at standard and fine settings with λ=0.001. This bound reflects the
observed model difference; it is not applied to synthetic truth recovery or
deliberately stronger penalties. Reference arrays are retained unchanged when
tolerances are tightened.

```python
import rexafs

bkg = rexafs.AUTOBK()
bkg.clamp_lambda = 0.001  # default; 0 disables the endpoint penalty
bkg.clamp_lo = 0
bkg.clamp_hi = 1
bkg.nclamp = 3
```

```rust
let mut bkg = rexafs::prelude::AUTOBK::new();
bkg.clamp_lambda = Some(0.001);
bkg.clamp_lo = Some(0);
bkg.clamp_hi = Some(1);
```

The JavaScript AUTOBK object exposes the same `clamp_lambda` property. In the
desktop app, select **Clamps & window → clamp model → Fixed λ**, then edit
**clamp λ**. An empty λ field uses 0.001.

## Compatibility

`Fixed` and `TwoPass` retain the historical direct-solver scale models and their
regularization/fallback behavior. They are distinct from `FixedPenalty`.
Iterative legacy solvers require a legacy clamp policy; the API rejects a
`FixedPenalty`/iterative-solver combination. The desktop selector switches the
paired solver/model setting to keep that combination valid.

Older saved projects with no clamp-model field retain the legacy `Fixed` model.
New projects explicitly save `FixedPenalty`, even when other defaults are omitted.
The parameter fingerprint, scoped copying, overrides, and undo history include
the model and λ. The fixed objective does not use the legacy dynamic-clamp Jacobian.
As of 0.2.4, both backends include the missing product-rule term in that legacy
Jacobian. Iterative legacy results can therefore change on recomputation;
the residual objective, frozen-scale direct models and fixed-λ default are
unchanged. See the [numerical compatibility record](fft-grid-compatibility.md).
