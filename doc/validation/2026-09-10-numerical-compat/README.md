# Numerical compatibility check, 2026-09-10

The baseline is the locally built 0.2.3 Python binding used for the preceding
Linux desktop review. The candidate is the optimized 0.2.4 binding with issue
#20's corrections. These files record available-host comparisons, not publication
artifacts. Platform: Ubuntu 24.04.4 ARM64, CPython 3.12.

[`record.py`](record.py) records complete arrays and input SHA-256 values for
measured Cu, Ni and Ru. Run it with the Python executable of each separately
installed binding, passing an output path. It uses each release's normalization
defaults, rbkg=1, k=0..12, kstep=0.05, NFFT=2048, and either the fixed-λ direct
solver or legacy LM with the `Fixed` policy. Clamps retain their defaults. The
public FFT uses Input mode and its historical defaults.

[`before.json`](before.json), [`after.json`](after.json) and
[`comparison.json`](comparison.json) retain the results. Every fixed-λ result
array, energy origin and grid is exactly equal. Legacy LM changes are:

| Spectrum | χ relative L2 difference | FFT magnitude relative L2 difference |
|---|---:|---:|
| Cu | 0.001429% | 0.0003572% |
| Ni | 0.0002597% | 0.0001075% |
| Ru | 0.00001779% | 0.00002324% |

These compare implementations; they do not measure recovery of experimental
truth. The full core and ndarray suites pass, including existing FEFF fit-space
comparisons, and the new independent FFT fixtures validate window values,
complex transforms, and fixed-distance shell amplitude/phase fits. See the
[method and migration policy](../../fft-grid-compatibility.md).
