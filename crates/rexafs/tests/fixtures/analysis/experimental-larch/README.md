# Experimental MBACK and wavelet comparisons

These cases use three **measured spectra already retained in this repository**.
No synthetic spectrum or unpublished ReGe measurement is used. The original
files are not duplicated or modified. `manifest.json` pins their paths, hashes,
source URLs, full original attribution records and the compressed reference
hashes. Tests read local files and make no network requests.

| Reference | Measurement | Samples | Data terms |
| --- | --- | ---: | --- |
| `cu-rt.json.gz` | Cu foil, room temperature, APS 13-ID-C | 408 | CC0-1.0 |
| `cu-10k.json.gz` | Cu foil, 10 K, NSLS X11A | 612 | CC0-1.0 |
| `ruo2.json.gz` | RuO₂, Ru K edge, Aichi SR BL11S2 | 4,369 | CC-BY-NC-SA-4.0; academic, noncommercial validation |

The Cu files come from [XASDataLibrary](https://github.com/XraySpectroscopy/XASDataLibrary/tree/284edcc1752ede0dd41c7e66eb2dbf6cf9589980/data/Cu),
whose [data license](https://github.com/XraySpectroscopy/XASDataLibrary/blob/284edcc1752ede0dd41c7e66eb2dbf6cf9589980/doc/license.rst)
is retained in `../../xas/LICENSES/XraySpectroscopy--XASDataLibrary--license.rst`.
Ru attribution: **Masashi Ishii; Aichi SR. XAFS spectrum of Ruthenium(IV) oxide**,
[NIMS MDR, DOI 10.48505/nims.3886](https://doi.org/10.48505/nims.3886).
Its terms are retained in `../../xas/LICENSES/CC-BY-NC-SA-4.0.txt`.
The derived Ru reference remains under those terms; it is not relicensed under
the software's MIT license. Each compressed file has a `.license` sidecar.
All of this directory and its integration test are excluded from crates.io.

## Inputs and explicit settings

Cu uses each file's stored `mutrans` signal without rescaling. The Aichi 9809
file uses its observed Bragg angle, recorded Si(311) spacing 1.63748 Å, and
`ln(I0/I1)`. Despite `-f` in its filename, the recorded mode is transmission.
No additional dark-current or gain correction is inferred. Tests verify the
native reader arrays against the independently converted reference arrays.

Energy is in eV. Fixed E₀ values are 8980 eV for Cu and 22140 eV for Ru, taken
from the source edge records; they are comparison settings, not a claim of
optimal experimental calibration. The Cu pre-edge interval is −150 to −40 eV
relative to E₀; Ru uses −250 to −50 eV. All post-edge intervals are +100 to
+800 eV. MBACK uses degree 2, positive scale and no erfc term. Larch's unchanged
`match_f2` objective is minimized by lmfit least squares with explicit tight
tolerances and `x_scale="jac"`; this compares the mathematical objective, not
Larch's interactive initialization. Its `preedge` function supplies the auxiliary
normalization convention. See [MBACK](../../../../../../doc/mback-normalization.md).

For wavelet validation, Larch prepares unweighted χ(k) from each measured μ(E)
using polynomial normalization and AUTOBK: Rbkg 1 Å, k 0–14 Å⁻¹, step 0.05 Å⁻¹,
weight 1, Hanning dk 0.1 Å⁻¹, FFT length 2048, three clamp points, low/high
clamps 0/1. The exact recipe and χ arrays are retained. **Both wavelet
implementations receive those same χ arrays**; this does not test AUTOBK parity.
The transform uses k weight 2 and the complete 130 × 281 complex map per case.
Larch couples Cauchy order to R row count and uses twice its `nfft` argument;
rexafs explicitly uses order 130, those exact R coordinates and FFT length 2048.
See [the wavelet conventions](../../../../../../doc/wavelet-analysis.md).

## Oracle and checks

The generator loads unchanged numerical functions from Larch revision
`e3c93284fed358c2c8979cba4c139430527433c6`; it removes only interactive decorators
and supplies simple group adapters. NumPy 2.3.2, SciPy 1.16.1, lmfit 1.3.4 and
xraydb 4.5.8 are pinned. SciPy's FFTPACK backend prepares χ; NumPy FFT computes
the Larch wavelet. Source function hashes and database version are recorded.
No rexafs code or output participates in generation. Larch's license is retained
in `LARCH-LICENSE.txt`; its wavelet source also retains the historical
Marne-la-Vallée, Hans-Argoul, Argoul-Munoz, Farges-Munoz and Newville attribution.

```sh
cargo test --locked -p rexafs --test experimental_analysis_reference
uv run scripts/generate-experimental-analysis-reference.py --output /tmp/new-experimental-reference
```

Generation requires network access to pinned Larch source, and refuses to
overwrite an existing directory. Compare any new generation before adopting it;
do not silently replace historical arrays. Compressed JSON retains every point,
including all real and imaginary cells, with deterministic gzip timestamps.

On macOS ARM64, 17 September 2026, all six experimental tests passed. Maximum
absolute normalized-MBACK differences were 1.49×10⁻¹⁰ (Cu RT), 3.65×10⁻⁹ (Cu 10 K)
and 6.82×10⁻⁹ (RuO₂). Across all compared MBACK curves the largest difference was
1.90×10⁻⁸; across all wavelet real/imaginary cells it was 5.61×10⁻¹⁴. Recorded
absolute tolerances are 10⁻⁶ for MBACK curves, 10⁻⁷ for scale, 10⁻⁹ for objective
and 2×10⁻¹⁰ for wavelet cells. These checks establish numerical agreement for
the stated settings, not physical accuracy or universal beamline qualification.
