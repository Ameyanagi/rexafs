# Synthetic Cauchy wavelet references

`larch-reference.json` records two analytic synthetic signals processed by the
unchanged numerical body of pinned Larch `cauchy_wavelet`, with only its interactive
group adapter/decorator replaced. It records source URL/SHA-256, commit, NumPy
version, original k/χ, actual FFT length, order, exact R coordinates and complex
arrays. No rexafs output participates in generating the reference.

[`generate-wavelet-reference.py`](../../../../../../scripts/generate-wavelet-reference.py)
reproduces the fixture. Larch's `nfft` argument is half its actual FFT length and
its order equals its R row count; tests explicitly match these conventions.
They do not claim identical application defaults. Independent direct DFT tests
provide a separate sign/scaling oracle.

`LARCH-LICENSE.txt` preserves the upstream MIT license. The reference function's
header additionally attributes the original 2000 code to Univ. Marne la Vallee,
France, with Hans–Argoul, Argoul–Muñoz and Farges–Muñoz algorithm/interface work
and Matthew Newville's 2014 Python translation. The scientific reference is
[Muñoz, Argoul and Farges (2003)](https://doi.org/10.2138/am-2003-0423).
The native rexafs implementation follows its documented fixed-order discrete
filter; historical Larch settings remain explicitly identified in these tests.
Project-authored synthetic inputs and generator follow the repository license.

Fixtures and their integration test are excluded from the crates.io archive.
Running Rust tests requires no network and reads no private experimental data.
