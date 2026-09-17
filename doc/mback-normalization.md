# MBACK normalization (unreleased)

MBACK matches an absorption measurement to an atomic reference while fitting a
smooth background outside the near-edge region. It provides an alternative to
conventional polynomial normalization, which remains the rexafs default. It does
not fix energy calibration, saturation or sample over-absorption. The method is
based on [Weng, Waldo and Penner-Hahn (2005)](https://doi.org/10.1107/S0909049504034193).

## Rust: one model, one operation

```rust
use rexafs::{MBack, Spectrum};
# fn example(energy: &[f64], mu: &[f64]) -> Result<(), rexafs::Error> {
let model = MBack::for_edge("Cu", "K")
    .pre_edge(-200.0..=-50.0)
    .post_edge(100.0..=800.0);
let result = model.fit(energy, mu)?;
// result.norm and result.flat are dimensionless owned arrays.
// result.fpp is the separately matched scattering factor.

// Use the same model in the ordinary spectrum processing pipeline:
let mut spectrum = Spectrum::from_arrays(energy, mu)?;
spectrum.set_normalization_method(model)?.normalize()?;
# Ok(())
# }
```

The arrays use energy in eV and a consistent input absorption scale. They must
be finite, matching, and strictly increasing in energy. The standalone fit borrows
and leaves both arrays unchanged. The spectrum setter takes the settings and
invalidates background/Fourier outputs; normalization then stores new results.
No database setup, network call or Python installation is required. Full results
are in the selected `MBack.result`; table provenance is retained automatically.

The default background degree is 2 and the erfc term is disabled. `.degree(0..5)`
changes background flexibility, not the auxiliary post-edge normalization degree.
`.e0(8979.0)` fixes the measured edge origin. Otherwise the existing derivative
edge detector resolves E₀ before fitting. E₀ stays fixed during MBACK; the atomic
table is not shifted to it. Calibrate the measured energy upstream when necessary.

All intervals are offsets from E₀. Explicit intervals must be fully covered and
stay on their respective sides of E₀. They are never shortened automatically.
Without explicit ranges, rexafs suggests the outer 80% of each available pre/post
span, bounded by the nearest other tabulated edge with a 10 eV margin. This is a
rexafs starting suggestion, not a universal physical fitting interval. Inspect
`result.pre_edge` and `result.post_edge` and revise them to exclude real near-edge
structure. Neighboring edges inside an explicit interval produce an error.

An optional fluorescence-background term needs an explicit emission selection
and finite width/amplitude bounds:

```rust
use rexafs::xafs::mback::MbackErfc;
use rexafs::MBack;
let model = MBack::for_edge("Cu", "K")
    .erfc(MbackErfc::new("Ka1", 500.0..=1500.0, 0.0..=10.0));
```

Here width is in eV and amplitude in f₂ units. These bounds illustrate syntax;
they are not recommended for every measurement. A narrow erfc far from the scan
can be indistinguishable from zero, while a broad one can resemble a polynomial.
Nonidentifiable fits fail. A selected emission must originate at the chosen
absorber edge. A family can be requested through `MbackErfc.emission`; it remains
distinct from an individual line and retains its contributing records. This term
models a smooth background; it does not correct fluorescence over-absorption.

## Objective and outputs

The implementation is [`mback/solver.rs`](../crates/rexafs/src/xafs/mback/solver.rs).
It uses the full objective in the [pinned Larch `match_f2` function][larch], not the
separate simplified `mback_norm` routine. For included point i:

```text
t_i = (E_i − E₀) / E_scale
B_i = sum(c_j * t_i^j, j=0..degree) + A * erfc((E_i − E_em) / xi)
r_i = (f₂_i + B_i − s * μ_i) / sqrt(N_region)
objective = sum(r_i²)
```

E, E₀, E_scale, E_em and xi are in eV. μ is input absorption, s is its fitted
positive conversion to f₂, and B, c_j and A have f₂ units. N_region counts included
pre-edge or post-edge samples; each region therefore contributes its average
squared residual. These weights balance regions, not measured inverse variances.
No statistical covariance or experimental confidence interval is inferred.
E_scale is the largest absolute fitted range offset; the scaled polynomial
coordinate improves numerical conditioning without changing its function space.
The erfc term is zero when disabled.

Without erfc, the solver uses column-scaled singular value decomposition (SVD),
rejecting relative singular values at or below 10⁻¹⁰. A nonpositive optimum scale
is rejected: its feasible boundary is not a usable normalization. With erfc, a
64-interval log-width grid identifies candidate minima. Bounded golden-section
refinement profiles those minima, refitting all linear coefficients at every
width and refitting remaining coefficients when amplitude reaches a bound.
The best checked candidate is retained. This deterministic bounded search is a
rexafs solver choice, not Larch's Levenberg–Marquardt implementation or a proof of
global optimality. Final Jacobian rank includes width. Bound-active and poorly
conditioned fits carry warnings; no clipped or rank-deficient solution is called
valid. Parameters and search bounds should be checked for scientific sensitivity.

The two absorption outputs serve different purposes:

```text
fpp_i = s * μ_i − B_i
Delta = Q(E_nearest_E₀) − P(E_nearest_E₀)
edge_step = Delta / s
norm_i = (s * μ_i − P_i) / Delta
```

P and Q are auxiliary pre-edge and post-edge curves fitted to `f₂+B`, using a
linear pre-edge, quadratic post-edge and zero Victoreen exponent. The operation
uses the saved resolved intervals and the existing rexafs conventional fit
endpoint convention (lower index at/below start; nearest upper index excluded).
The edge step is evaluated at the nearest measured E₀ sample. Unlike historical
polynomial normalization, MBACK rejects a nonpositive or numerically singular
Delta instead of accepting a floored step. The stored step is an output; the
legacy `set_edge_step` API does not override MBACK's atomic normalization.

`flat` subtracts `(Q_i−P_i)/Delta−1` from norm at and above the nearest E₀ sample;
below it flat equals norm. This is the recorded `rexafs_prepost_linear_quadratic_v1`
convention. Matched fpp, norm and flat must not be interchanged without naming the
representation. Polynomial MBACK degree and auxiliary quadratic degree are separate.

Results retain requested/resolved intervals, E₀, reference identity, polynomial
coordinate and coefficients, emission/bounds, fit indices/weights, full curves,
objective, rank, scaled condition number and evaluation count. Passing
`.reference(saved_result.reference.clone())` requires that exact table on replay.
Saved historical arrays remain readable when recalculation is unavailable.
Empty historical MBACK placeholders still deserialize; they require absorber/edge
selection before they can calculate new output. A failed standalone normalization
clears its cached arrays instead of leaving a stale successful result visible.

## Reference data and validation

The [offline data record](../crates/rexafs/data/atomic/README.md) documents XrayDB,
licenses, checksums, interpolation and coverage. Neither runtime tests nor fitting
download data. `scripts/generate-mback-reference.py` is a separate fixture generator:
it loads only named numerical functions from a pinned Larch commit, saves their
hashes, and records NumPy, SciPy, lmfit and XrayDB versions. Its synthetic inputs
include irregular spacing, unequal region counts, excluded near-edge structure
and both erfc modes. Optimizer starts/bounds are explicitly matched; this is a
numerical comparison with Larch's full objective and auxiliary convention, not
Larch's interactive default initialization. All generated reference data stay in
the Git repository and are excluded from crates.io.

The Python and TypeScript constructors now share the native implementation:

```python
from rexafs import MBack
model = MBack("Cu", "K", pre_edge=(-200, -50), post_edge=(100, 800))
result = model.fit(energy, mu)
spectrum.set_normalization_method(model).normalize()
```

```ts
import { MBack } from "rexafs/node";
const model = new MBack("Cu", "K", {pre_edge: [-200,-50], post_edge: [100,800]});
const result = model.fit(energy, mu); // Float64Array inputs
spectrum.set_normalization_method(model).normalize();
model.free();
```

Both return `result.norm`, `result.flat` and diagnostics. Python arrays are copied
on property access; JavaScript arrays are owned copies. `spectrum.mback_result()`
returns the latest independent full result, or None/undefined after invalidation.
`result.definition` creates a replay model pinned to the original atomic table;
JavaScript callers must free that model when finished. Browser construction needs
`await init()` and large synchronous fits should run in a Worker. `MbackErfc`
takes named `width` and `amplitude` bounds in both languages. The installed-package
checks compare both erfc modes with the same Larch fixture and exercise editor help.

GUI method comparison, retained normalization history and series/Live workflow
are still being integrated. These increments do not establish those workflows or
physical qualification on an experimental sample.

[larch]: https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/larch/xafs/mback.py
