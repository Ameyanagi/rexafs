# Offline atomic references (unreleased)

rexafs uses the exact `xraydb = 0.4.1` Rust dependency, whose embedded compressed
resource contains XrayDB database version 9.2. This is a runtime scientific
resource, unlike the repository-only beamline fixtures. No network connection,
SQLite service or Python installation is needed. The dependency already supplies
the resource; this directory preserves the licenses and provenance without a
second database copy.

[`provenance.json`](provenance.json) records the compressed artifact's source,
size (3,187,870 bytes) and SHA-256. The decoded xraydb-data 0.4.1 record, streamed
as compact serde JSON in struct/vector order, has SHA-256
`fb29697588bd24ffafac8e2d5bfcf808df9a66b05987ef35b1224d3b8fb7c67e`.
The provider fingerprints the actual loaded record, including custom upstream
initialization. It reports the greatest numeric version tag rather than the
upstream helper's first historical version entry. Historical results require
their exact provider, data checksum and table profile to be available before
recomputation. Reading an old result does not require recomputation.

The Rust provider is MIT OR Apache-2.0; both notices are retained here. XrayDB
dedicates its compiled database to CC0, while explicitly noting that some original
datasets have no clear copyright/license statement. Preserve that qualification
in [`XrayDB-LICENSE.txt`](XrayDB-LICENSE.txt); do not describe it as a new license
for every original source. No XrayDB Python code is embedded in rexafs.

## Numerical contracts

The implementation is [`atomic/mod.rs`](../../src/atomic/mod.rs).

- `ChantlerF2LogLogV1`: linear interpolation in log energy and log f₂ on the
  original Chantler grid, including near-edge tabulation. Values are dimensionless
  electron units. No chemical shift or experimental broadening is applied.
  See [Chantler (2000)](https://doi.org/10.1063/1.1321055).
- `ElamTotalV1`: photoelectric plus coherent plus incoherent mass attenuation,
  in cm²/g, using Elam's stored log-energy/log-value spline coefficients. The
  dependency's knot convention selects the lower interval at an exact edge.
  See [Elam, Ravel and Sieber (2002)](https://doi.org/10.1016/S0969-806X(01)00227-4).
- `ElamTransitionsV1`: edge and emission-line energies in eV. Individual lines
  remain distinct from within-shell intensity-weighted families.

f₂ queries must remain in the selected element's Chantler grid; Elam queries
must be within 100–800,000 eV. Unsupported elements/energies return errors instead
of clamping or extrapolating. Coverage is operation-dependent. Compound total
attenuation is `sum(w_j * a_j)`, where `w_j = n_j M_j / sum(n_k M_k)`, `n_j` is
atom count and `M_j` molar mass in g/mol. No density is used. The supported formula
grammar has canonical symbols, positive decimal counts and nested parentheses.
Use `CuSO4(H2O)5` for that composition. Isotopes, charge, hydrate separators, named
materials and scientific notation are rejected with a byte position; a decimal
point is always part of an atom count, never a hydrate separator.

`scripts/generate-atomic-reference.py` records independent XrayDB 4.5.8 queries
for four edges and four compounds, including points immediately around an edge.
Its SQLite checksum and versions are in the repository-only fixture. The test
requires relative agreement within 2×10⁻¹⁰; this checks implementation agreement,
not uncertainty or physical accuracy of the reference tables. The Rust provider's
explicit rejection policy is tested separately.
