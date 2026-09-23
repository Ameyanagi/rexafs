---
title: "Licenses and example data"
description: "Project attribution, packaged notices and measurement provenance."
audience: user
---

## rexafs

rexafs is available under the [MIT license](/LICENSE-MIT.txt) or
[Apache License 2.0](/LICENSE-APACHE.txt), at your option. Copyrights remain with
its contributors. For dependency and calculation-engine notices, see
**Help → Licenses** in the desktop.

## Browser scattering engine

The scattering workspace distributes [ReFEFF 0.4.0](https://github.com/Ameyanagi/refeff/releases/tag/v0.4.0),
a Rust port derived from FEFF10. Its [license and FEFF10 conditions](/refeff/LICENSE)
and [provenance notice](/refeff/NOTICE.md) remain applicable. The browser
filesystem adapter retains its [MIT](/refeff/browser-wasi-shim-LICENSE-MIT) and
[Apache-2.0](/refeff/browser-wasi-shim-LICENSE-APACHE) notices.
Retained [Rust dependency notices](/refeff/RUST-NOTICES.txt) and their
[source inventory](/refeff/rust-dependencies.json) accompany the compiled engine.
Its runtime attribution includes [toolchain notices](/refeff/TOOLCHAIN-NOTICES.txt),
Rust's [standard-library notice](/refeff/rust-COPYRIGHT-library.html) and
[source provenance](/refeff/toolchain-provenance.json).

The [ZnSe input](/refeff/znse.inp) is copied unchanged from
[ReFEFF's 0.4.0 test fixture](https://github.com/Ameyanagi/refeff/blob/v0.4.0/crates/refeff/tests/data/znse.inp).
This historical test input includes a krypton (Kr) scatterer in the first shell;
it is a calculation example, not a measurement or pure ZnSe model.
The [runtime manifest](/refeff/manifest.json)
records the upstream source, release archive and file hashes.

## Cu example

[cu_150k.xmu](/examples/cu_150k.xmu) contains a Cu foil
measurement at 150 K from NSLS X-11A, September 1992. It is retained without
numerical changes from the XrayLarch example collection at revision
`d8678dd666fd95839fe9dc71b4dbe8bedec278ff`. The header also identifies its UWXAFS
3.0 distribution history. Retain that header when redistributing the example.

[Source and provenance](https://github.com/xraypy/xraylarch/blob/d8678dd666fd95839fe9dc71b4dbe8bedec278ff/examples/xafsdata/cu_150k.xmu)
· [Retained provenance record](/examples/PROVENANCE.txt).

## Synthetic copper reduction example

From 0.2.13, the desktop bundles three measured CuO, Cu₂O and Cu references and
50 deterministic mixtures of their raw absorption. The data owner supplied the
group's measurements in `Cu oxides.prj` and requested this teaching example's
distribution. The original Athena project is not redistributed. Its checksum,
source labels and generation parameters are retained in the bundled project's
metadata; no additional acquisition details or upstream data license are inferred.
The software's MIT/Apache license is not presented as a license for the original
measurements.

The [tutorial](/docs/desktop/synthetic-copper/) explains the recipe, automatic
processing and interpretation. The [example provenance](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/crates/rexafs-gui/data/examples/README.md)
retains the distribution request. This 50-frame example is separate from the
historical 100-mixture test fixtures described below.

## Documentation screenshots

<span id="desktop-0214-release-captures"></span>

### Desktop 0.2.14 release captures

Eight full, unedited PNG windows in `screenshots/0.2.14/` document the signed and
notarized Apple Silicon **0.2.14** release installed from its DMG on 24 September
2026 in Japan. The [manifest](/screenshots/0.2.14/capture.json) records source,
signing, installer, executable, input and image hashes, and each original size:
2384 × 1536 for main windows, 740 × 1216 for the separate monitor and 820 × 592
for the axis dialog. No cropping, resizing, annotation or replacement plots
were applied.

The Computer Use connector could not authenticate (error −10000). This review
used existing macOS Accessibility permissions, native controls and native window
capture instead; these images are not connector output. The
[signed workflow record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-24-release-0.2.14/signed-workflow.md)
retains that limitation and the completed checks.

The Cu project repeats a prepared 517-point reference derived from the
user-provided 2016 Athena Cu-oxide project. Its
[attribution record](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/crates/rexafs/tests/fixtures/analysis/cu-mixtures/README.md)
retains permission and the unknown original acquisition author/license; the
measurement is not MIT-licensed code. Three separate synthetic XDI sources
exercise transmission, fluorescence and reference averages. These captures and
software comparisons do not establish an experimental time trend or improved
signal-to-noise. Earlier captures below retain their original provenance.

### Desktop 0.2.13 release captures

The six images in `screenshots/0.2.13/` are full, unedited **1192 × 768**
Computer Use windows from the signed and notarized Apple Silicon **0.2.13**
release installed from its DMG on 22 September 2026 in Japan. The
[capture manifest](/screenshots/0.2.13/capture.json) records the exact source,
build, signing run, installer, executable, bundled input and image hashes.
No cropping, resizing, annotation or replacement plots were applied.

PCA and MCR-ALS were calculated afresh from the 50-frame example described above,
using ordinary automatic processing and the tool defaults. The saved PCA model
and MCR result matched all 92,709 checked numerical values from the tagged-source
tutorial calculation exactly. Labels were excluded from that numerical comparison.
The Series image shows frame 50 minus frame 1; the Storage image records a scan
without deletion. These are teaching and software checks, not an experimental
validation of component identity or concentration accuracy.

<span id="desktop-0212-release-captures"></span>

### Desktop 0.2.12 release captures

The eight images in `screenshots/0.2.12/` are full, unedited **1192 × 768**
computer-use windows from the signed and notarized Apple Silicon **0.2.12**
release installed from its DMG on 20 September 2026. The
[capture manifest](/screenshots/0.2.12/capture.json) records source, signing,
artifact, executable, input and image identities. No cropping, resizing or
replacement plots were applied.

All views use the unchanged public room-temperature Cu measurement from the
[X-ray Absorption Data Library](https://github.com/XraySpectroscopy/XASDataLibrary/blob/284edcc1752ede0dd41c7e66eb2dbf6cf9589980/data/Cu/cu_metal_rt.xdi),
released under [CC0-1.0](https://github.com/XraySpectroscopy/XASDataLibrary/blob/284edcc1752ede0dd41c7e66eb2dbf6cf9589980/doc/license.rst).
Attribution: International X-ray Absorption Society and the credited data
contributors. RMC uses the built-in fcc Cu structure with 32 periodic sites and
ten attempts, fixed S₀² = 1 and ΔE₀ = 0 eV. It is a fresh software and persistence
check with amplitude mismatch remaining, not a calibrated or converged fit.
The Cu–Cu curves use periodic density and shell normalization; variance is not
uncertainty. No private measurement project is included.

The earlier source-candidate and historical release captures below retain
their original provenance.

<span id="desktop-0212-candidate-captures"></span>

### 0.2.12 candidate controls

The images in `screenshots/next/0.2.12/` are full, unedited computer-use captures
from the optimized source candidate containing PRs #103–#106, captured on
20 September 2026. They are not captures of a published signed 0.2.12 package.
The [manifest](/screenshots/next/0.2.12/capture.json) records the source identity,
executable and input hashes, dimensions and scope. No images were cropped,
resized, annotated or replaced with simulated plots.

Publication and normalization show the bundled Cu measurement described above.
The surrounding copied review project includes explicitly labeled synthetic
groups. Structural plots reopen a four-generation synthetic finite Cu–O hybrid
checkpoint. Those neighbor counts illustrate the controls; they are not bulk
g(r), an experimental fit, physical time or evidence of convergence. The active
Cu spectrum differs from the saved synthetic problem, so the app correctly
displays its saved-input warning. The source measurements were not changed or
newly redistributed for these captures. Earlier release captures retain their
original provenance below.

<a id="desktop-0211-workflow-captures"></a>

### Desktop 0.2.11 workflow captures

The homepage and current guides use 29 full, unedited **1192 × 768** window
captures made through `cua_repl` on **19 September 2026**, using the signed and
notarized macOS ARM64 **0.2.11** release. The
[workflow capture manifest](/screenshots/0.2.11/workflow-capture.json) records
the release, executable checksum, input provenance, settings and image hashes.
The original historical captures and attribution below remain available.

Import, processing, fitting and the two-frame Series use the public
room-temperature and 10 K Cu foils credited below under CC0. The
[downloadable walkthrough input](/examples/cu_metal_rt.xdi) retains its original
bytes and [source/license record](/examples/cu_metal_rt.xdi.license).
The two acquisitions do not constitute a controlled temperature experiment.
The one-shell ReFEFF fit uses an 8 Å Cu cluster, k = 2–12 Å⁻¹, R = 1–3 Å
and weight 2; it illustrates the workflow rather than a unique structural model.

Collection views reopen the saved 0.2.9 PCA/MCR results for the previously
authorized 100 synthetic Cu mixtures. Their limited academic-use provenance
is retained below; no unrestricted license is asserted for the source standards.
Reference comparison, component-to-group Fourier processing and all-frame LCF
were checked in 0.2.11. No original private project is distributed. The import
examples use Larch's MIT-distributed Athena project, the MIT-noticed xasref Mo
foil and the CC BY 4.0 PbTe measurement credited below. Three generated numeric
tables demonstrate pending-file dismissal; they are not measurements.
No Assistant message was sent and no access permission was changed.

### Desktop 0.2.11 RMC captures

The RMC guide includes three original, unedited **1192 × 768** JPEG window
captures made through the computer-use connector on **19 September 2026**.
They show the signed and notarized ARM64 **0.2.11** app installed from its DMG,
commit `70f61e81262f9ac593f813c895ed843aa23ee640`. The
[capture manifest](/screenshots/0.2.11/capture.json) records build and signing
workflows, archive and executable checksums, input attribution and image hashes.

The unchanged public room-temperature Cu measurement is from the International
X-ray Absorption Society's X-ray Absorption Data Library, with the source and
CC0 notice linked below. The built-in Cu structure uses a 32-atom supercell.
The ten-attempt run illustrates worker controls, periodic theoretical energy
updates with fixed amplitude and project reload. It reached the deliberately
narrow energy bound and did not establish convergence or physical accuracy.
No unpublished measurements or private project is included.

<a id="desktop-0210-workflow-captures"></a>

### Desktop 0.2.10 workflow captures

The historical 0.2.10 set retains eleven full, unedited
**2880 × 1800** JPEG window captures made through native macOS accessibility
controls and window capture on **18 September 2026**. They show the ARM64
**v0.2.10 tagged CI build**, commit `0b3aca3331e38ac4a34364824a74e96271f5ca1d`,
from [Release builds run 35316272629](https://github.com/Ameyanagi/rexafs/actions/runs/35316272629).
This is the CI archive before release signing and publication; the captures do
not certify the downloaded, signed package. The
[capture manifest](/screenshots/0.2.10/capture.json) records the archive,
executable, input and image checksums, settings and checks performed.

The inputs are the unchanged public
[room-temperature Cu foil](https://github.com/XraySpectroscopy/XASDataLibrary/blob/284edcc1752ede0dd41c7e66eb2dbf6cf9589980/data/Cu/cu_metal_rt.xdi)
and [10 K Cu foil](https://github.com/XraySpectroscopy/XASDataLibrary/blob/284edcc1752ede0dd41c7e66eb2dbf6cf9589980/data/Cu/cu_metal_10K.xdi)
measurements from the International X-ray Absorption Society's X-ray Absorption
Data Library and its credited contributors, distributed under the collection's
[CC0 data notice](https://github.com/XraySpectroscopy/XASDataLibrary/blob/284edcc1752ede0dd41c7e66eb2dbf6cf9589980/doc/license.rst).
The headers retain the original beamline and acquisition attribution.
Series uses these two separate measurements to demonstrate controls; it is not
a time series or a controlled temperature comparison. No unpublished ReGe data
or private project is included. No Assistant message was sent and no access
permission was changed.

### Earlier release captures

The retained 0.2.9 import, Assistant, collection-analysis and Series examples include
eight full, unedited 1192 × 768 JPEG captures made through computer use on
16 September 2026 from the signed and notarized macOS ARM64 **0.2.9** release.
The [release qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-16-release-0.2.9/review.md)
and [capture manifest](/screenshots/0.2.9/capture.json)
record software, inputs, checksums, settings and observed results.

The import capture uses Larch's unchanged
[`json_unzipped.prj`](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/examples/xafsdata/AthenaProjectFiles/json_unzipped.prj),
distributed under its repository's
[MIT notice](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/LICENSE).
All four project records were imported. The Assistant captures show its menus
beside the prepared Cu foil reference; no message was sent and no access
permission was changed for these captures.

The analysis captures use the requested **100 synthetic, noise-free Cu mixtures**
and prepared Cu foil, Cu₂O and CuO references. They are academic test data, not
100 new measurements. The original user-supplied Athena project remains private.
Its acquisition author, public source and redistribution license have not been
established; the library's MIT/Apache license must not be inferred for those
measurements. The repository retains the generated test fixtures with the
user's permission and excludes them from published packages. The
[fixture provenance](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/crates/rexafs/tests/fixtures/analysis/cu-mixtures/README.md)
records the available attribution and preparation. The figures illustrate
software behavior and do not establish unique chemical recovery.

The original desktop walkthroughs show full, unedited window captures from the published
macOS ARM64 0.2.4 package, captured through computer use on 13 September 2026
with the Cu example and built-in Cu structure. The fitting
walkthrough uses ReFEFF, an 8 Å cluster and one first-shell path. Values in these
screenshots describe that demonstration, not a benchmark or universal fit result.

The historical Next measurement reader set retains
full, unedited 1187 × 768 captures made through computer use on
14 September 2026 from an unreleased macOS ARM64 source build on
`test/beamline-fixtures`. These images show the EX3 import preview and original
header. The source measurement is Masashi Ishii and the Industrial Application
and Partnership Division's [XAFS spectrum of Lead telluride](https://doi.org/10.48505/nims.3178),
[MDR record](https://mdr.nims.go.jp/datasets/3c5953dc-1faa-47a7-a3f0-8211703adb58),
licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
The measurement bytes are unchanged; the screenshots visualize its stored energy
and absorption, without additional import corrections. The
[capture record](https://github.com/Ameyanagi/rexafs/tree/45ccd5e72eedf07d4e6f15b8c1d0e0e9661d0d86/doc/validation/2026-09-14-measurement-import-ui)
identifies the input and executable by checksum. These images describe unreleased
behavior and do not replace the versioned release screenshots above.

The historical Next set also retains the multiple-signal import controls in a full,
unedited 1187 × 768 capture from 14 September 2026. Its 651-point
`Mo foil 0001-r0003.dat` input comes from Ryuichi Shimogawa and contributors'
[xasref collection](https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/foil_QAS_sample_position/Mo%20foil%200001-r0003.dat),
with the collection's [MIT distribution notice](https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/LICENSE).
The original measurement bytes are unchanged. The screenshot plots the selected
reference signal as `ln(it / ir)` without normalization or detector corrections.
The repository's `doc/validation/2026-09-14-named-multisignal-import-capture.json`
records the input, executable and image checksums.

The retained historical Assistant images contain three full, unedited
1192 × 768 JPEG captures made through computer use on 15 September 2026 from
the signed and notarized macOS ARM64 0.2.7 app. They show its compact menus beside
the normalization parameters and bundled `cu_150k.xmu` example. No assistant
message was sent. Model availability depends on the connected installation.
The [0.2.7 qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-15-release-0.2.7/review.md)
identifies the original release build, signed executable and image checksums.
The Cu measurement retains the source attribution above; its original bytes are
unchanged. Older screenshots retain their original version labels.

The Assistant guide links to two historical, unedited 1187 × 768 JPEG captures
from an unreleased macOS ARM64 source build on `fix/assistant-context-size`,
captured through computer use on 15 September 2026. They show actual in-app AI
LCF and PCA calculations with generated mathematical spectra. No experimental
chemical standards were used in these two images. The source checkout's
`doc/validation/2026-09-15-assistant-context/review.md` records the signal recipe,
calculated results and capture hashes. These are separate from the released
0.2.7 screenshots and do not describe the published 0.2.8 packages.

The retained historical whole-project import image is a full, unedited
1187 × 768 JPEG captured through computer use on
15 September 2026 from an unreleased macOS ARM64 source build on
`fix/import-all-scans`. It shows all four records from Larch's
[`json_unzipped.prj`](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/examples/xafsdata/AthenaProjectFiles/json_unzipped.prj),
distributed under the repository's
[MIT notice](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/LICENSE).
The preview displays stored absorption without further correction. Original
measurement bytes are unchanged. The source checkout's
`doc/validation/2026-09-15-import-all-scans/capture.json` records the input,
source files, executable and screenshot checksums. The capture describes
unreleased behavior, not the published 0.2.8 packages.

## Scientific citations

Use the [citation guide](/docs/science/references/) for algorithm references.
Cite your measurement, structure source, FEFF backend and software version.

### Development Series workflow screenshots

`series-add-trend.jpg`, `series-trend-overview.jpg` and
`series-difference-colors.jpg` in `screenshots/next/` were captured through
computer use from the local development build: the trend editor and difference
color menu on 2026-09-17, and the saved-trend overview on 2026-09-16.
All displayed spectra are generated by
[`scripts/generate-series-metric-example.py`](https://github.com/Ameyanagi/rexafs/blob/feature/complete-analysis/scripts/generate-series-metric-example.py).
They illustrate a 513-frame series with a deliberately larger peak at frame 258.
They contain no unpublished experimental data. These screenshots document
unreleased behavior and do not replace the 0.2.9 captures.

### Development pending-import screenshot

`screenshots/next/pending-import-skip.jpg` is an unedited 1187 × 768 capture made
through computer use on 17 September 2026 from the local development build.
All signals were generated for software testing; they are not measurements.
The reproducible generator and validation record are in
[`doc/validation/2026-09-17-pending-imports`](https://github.com/Ameyanagi/rexafs/tree/feature/pending-import-controls/doc/validation/2026-09-17-pending-imports).
The image shows one accepted synthetic spectrum and five pending sources.
No unpublished experimental data or private project files are included.
