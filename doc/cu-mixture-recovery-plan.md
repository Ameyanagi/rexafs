# Copper mixture recovery and native MCR-ALS

Status: core and desktop experiment implemented and locally validated; unreleased,
16 September 2026. See the [validation record](validation/2026-09-16-cu-mixtures/README.md).
Work starts from
`dev` commit `44c6bc45817dad5df0f928d30a12f1e865d8cb0b` on
`test/cu-mixture-recovery`.

1. Read the user-supplied `Cu oxides.prj` without changing it. Preserve its
   checksum, original group identities and available historical attribution.
   Normalize Cu foil, Cu₂O and CuO once using recorded settings. Preserve energy
   positions and interpolate linearly onto common measured coverage.
2. Generate 100 noise-free mixtures with a recorded NumPy PCG64 seed and
   Dirichlet(1, 1, 1) fractions. Store individual XDI files, prepared standards,
   exact fractions and checksums as source-checkout fixtures, excluded from
   packages. User permission to retain derived tests does not establish an
   upstream open-data license; record that distinction explicitly.
3. Implement multivariate curve resolution by alternating least squares
   (MCR-ALS) in Rust, using existing nalgebra and constrained least-squares code.
   Expose nonnegative concentrations, optional sum-to-one closure, optional
   nonnegative spectra, explicit known-composition anchors, deterministic
   initialization and termination information. Require common measured coverage;
   do not extrapolate. The initial experiment used explicitly prepared arrays;
   the later API refinement below adds documented preparation of missing stages
   on copies. Check each subproblem's optimality
   rather than assuming the existing solver's iteration cap means convergence.
4. Compare native LCF, PCA and MCR-ALS against composition truth and an
   independent Python reference. Test blind MCR separately from a control that
   appends three explicitly labeled pure samples. The blind solver must never
   receive the generating standards or true fractions.
5. Add MCR-ALS beside LCF/PCA in the Data analysis tools, with an asynchronous
   calculation, component plot, numerical results and analysis export. Inspect
   the actual release GUI with computer use and retain screenshots and results.

## Model and interpretation

For `n` spectra at `p` energies, the dimensionless normalized absorption matrix
`D` has shape `n × p`. Approximate it by `C S + R`, where `C` is the `n × r`
coefficient matrix, `S` contains `r` component spectra as rows, and `R` contains
residuals. Alternating least squares holds one factor fixed while solving the
other. Nonnegative coefficients summing to one describe convex mixtures of
compatible normalized standards; they are not automatically mass fractions.
Spectra may be signed because baseline subtraction can produce small negative
values. Report SSE (sum of squared residuals) and SSE / sum(D²), neither of which
is a statistical confidence level. This follows the bilinear model in the
[NIST pyMCR paper](https://doi.org/10.6028/jres.124.018) and
[official implementation documentation](https://pages.nist.gov/pyMCR/).

PCA with no mean subtraction should have rank three for independent standards;
centering closed three-component mixtures gives at most two varying directions.
PCA directions are not pure chemical spectra. MCR can reconstruct the data while
its factors differ from the originals: permutation, scale and rotational
ambiguities must be distinguished from reconstruction error. See the method
authors' [MCR-Bands explanation](https://mcrals.wordpress.com/theory/mcr-bands/).
Known pure samples are additional information in the anchored control, not a
claim of blind identification.

pyMCR 0.5.1 is a development comparison dependency only. The native desktop
implementation will not embed Python or copy pyMCR source. Broader Components
workspace design and binding APIs remain separate work from this experiment.

## Expanded desktop workflow

The [analysis guide](cu-mixture-analysis.md) records the flat calculation settings,
PCA count diagnostics, range presets, all-spectrum plotting, reference matching
and calculated groups with provenance added after reviewing the notebooks.

## Shared analysis API follow-up

Implemented in the same unreleased branch: missing normalization/background
stages run on copies with the input settings; Norm remains the core default and
Flat is explicit. LCF, PCA and MCR require finite increasing intervals fully
covered by each input. Energy intervals remain offsets from E₀. Series LCF uses
the core batch API; PCA suggestions and reconstruction-error curves also come
from the core. Tests compare automatic and explicit preparation and preserve
input arrays, settings and caches. See the [API guide](analysis-api.md).

Remaining todo:

- [ ] Public Python LCF/batch LCF/PCA/MCR bindings and NumPy result objects.
- [ ] Equivalent TypeScript functions, options and dimensioned result arrays.
- [ ] Binding parity tests and editor-visible documentation for defaults,
  automatic preparation, range errors, batch failures and count diagnostics.
