# Copper mixtures for component-analysis tests

These are **100 synthetic, noise-free mixtures**, not 100 new measurements.
They derive from Cu foil (`cufoil_abs`), Cu₂O (`cu2o_abs`) and CuO (`cuo_abs`)
in the user-supplied `Cu oxides.prj`. The source was written by Athena 0.8.061
on 12 September 2016. Its SHA-256 is
`18684eb2c4776d5d3661f6ad6fdfae6fb81d91571bda00611ec33bd686c67dc6`.
The original project is unchanged and is not copied here. The historical foil
title records `NORMAL: E0 shift = 7.79`; the experiment adds no energy shifts.

## Permission and attribution

The user explicitly authorized retaining the generated 100 spectra as repository
test fixtures on 16 September 2026. The original collection's acquisition author,
public source URL and redistribution license have not been established from the
supplied project or its neighboring files. Do not infer an author from a historical
Windows pathname or apply the library's MIT/Apache license to these data. This
record preserves the available attribution without inventing an upstream license.
This collection is retained for the requested academic test; it is excluded from
published crates and is not an assertion of unrestricted upstream reuse rights.

## Preparation

`manifest.json` records settings, versions, source identities and per-file hashes.
Using rexafs 0.2.8, each original spectrum is normalized once with E0 = 8979 eV,
a linear pre-edge fit from E0 −150 to −75 eV, a quadratic post-edge fit from
E0 +150 to +650 eV, and Victoreen exponent zero. E0 is a common normalization
reference and does not shift the measured energy axes. Absorption is expressed
as `(mu − pre_edge) / edge_step`; the edge step comes from the fitted baselines.
These settings are an explicit experimental choice, not an exact replay of the
historical Athena project. Slight negative baseline values are retained.

Prepared standards are linearly interpolated onto the foil's 517 samples within
all three sources' measured coverage (8780.206–9768.204 eV). No extrapolation,
smoothing, flattening, edge alignment or additional noise is used. NumPy PCG64
seed 20260916 draws 100 Dirichlet(1, 1, 1) coefficient rows. Each mixture is the
weighted sum of the prepared standards; `truth.json` stores its exact generating
fractions in Cu foil, Cu₂O, CuO order. Fractions are coefficients of normalized
absorption, not a claim about physical mass composition.

LCF/PCA/MCR validation uses 8950–9150 eV. Uncentered PCA has three independent
directions; mean-centered closed mixtures have two. Blind MCR receives only the
100 mixtures. The anchored control additionally receives three explicitly labeled
pure rows. These controls distinguish reconstruction from chemical identification.
See [the implementation plan](../../../../../../../doc/cu-mixture-recovery-plan.md).

Regenerate into a **new** directory with the checksum-matched local source:

```sh
python scripts/generate-cu-mixtures.py '/path/to/Cu oxides.prj' /tmp/cu-mixtures-new
cargo test --locked -p rexafs --test cu_mixture_recovery
```

The generator is [`scripts/generate-cu-mixtures.py`](../../../../../../../scripts/generate-cu-mixtures.py).
Tests read committed files without a source checkout, network access or Python.
Results may be exported by setting `REXAFS_CU_REPORT` to a local output directory.
