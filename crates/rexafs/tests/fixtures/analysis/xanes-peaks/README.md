# Synthetic XANES peak-fit reference

`lmfit-reference.json` is generated entirely from analytical profiles and seeded
Gaussian noise by [generate-peakfit-reference.py](../../../../../../scripts/generate-peakfit-reference.py).
No experimental or unpublished measurements are included. The generated data
and generator are project-authored under the repository's MIT OR Apache-2.0
license. No lmfit source code is copied into the fixture.

Regenerate with `uv run scripts/generate-peakfit-reference.py`. The script pins
Python 3.12, NumPy 2.3.2, SciPy 1.16.1 and lmfit 1.3.4; exact runtime versions
are retained inside the JSON. Reference definitions come from the
[official lmfit documentation](https://lmfit.github.io/lmfit-py/builtin_models.html).
Rexafs runtime and ordinary Cargo tests require none of these Python packages.

Each of four shapes (Gaussian, Lorentzian, pseudo-Voigt and Voigt) is jointly
fitted with a linear baseline. Data have 251 irregularly spaced absolute-energy
points, E₀ = 9000 eV, and known independent errors in arbitrary absorption units.
The fit interval is −15 to +35 eV relative to E₀, excluding +9 to +10 eV.
The deterministic PCG64 seed is 20260917. The reference uses lmfit's
`least_squares` method with absolute covariance scaling; it records fitted arrays,
parameters, covariance, errors, point indices and objective.

The generator declares acceptance tolerances before comparison: 10⁻⁴ absolute
parameter difference, 2×10⁻⁷ maximum model-array difference, 10⁻⁶ objective
difference and 3×10⁻⁴ relative standard-error difference. Width conversions are
explicit in the generator. Agreement tests mathematical compatibility for these
synthetic examples; it does not validate a chemical interpretation.

Run `cargo test --locked -p rexafs --test peak_fitting`. This integration test and
the entire analysis-fixture directory are excluded from the crates.io archive.
