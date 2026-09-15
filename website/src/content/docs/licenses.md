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

## Documentation screenshots

The original desktop walkthroughs show full, unedited window captures from the published
macOS ARM64 0.2.4 package, captured through computer use on 13 September 2026
with the Cu example and built-in Cu structure. The fitting
walkthrough uses ReFEFF, an 8 Å cluster and one first-shell path. Values in these
screenshots describe that demonstration, not a benchmark or universal fit result.

The [Next measurement reader guide](/docs/reference/next/measurement-reading/)
adds full, unedited 1187 × 768 captures made through computer use on
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

The current Next guide also shows the multiple-signal import controls in a full,
unedited 1187 × 768 capture from 14 September 2026. Its 651-point
`Mo foil 0001-r0003.dat` input comes from Ryuichi Shimogawa and contributors'
[xasref collection](https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/foil_QAS_sample_position/Mo%20foil%200001-r0003.dat),
with the collection's [MIT distribution notice](https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/LICENSE).
The original measurement bytes are unchanged. The screenshot plots the selected
reference signal as `ln(it / ir)` without normalization or detector corrections.
The repository's `doc/validation/2026-09-14-named-multisignal-import-capture.json`
records the input, executable and image checksums.

The [Assistant guide](/docs/desktop/assistant/) shows three full, unedited
1192 × 768 JPEG captures made through computer use on 15 September 2026 from
the signed and notarized macOS ARM64 0.2.7 app. They show its compact menus beside
the normalization parameters and bundled `cu_150k.xmu` example. No assistant
message was sent. Model availability depends on the connected installation.
The [0.2.7 qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-15-release-0.2.7/review.md)
identifies the original release build, signed executable and image checksums.
The Cu measurement retains the source attribution above; its original bytes are
unchanged. Older screenshots retain their original version labels.

The Assistant guide also includes two full, unedited 1187 × 768 JPEG captures
from an unreleased macOS ARM64 source build on `fix/assistant-context-size`,
captured through computer use on 15 September 2026. They show actual in-app AI
LCF and PCA calculations with generated mathematical spectra. No experimental
chemical standards were used in these two images. The source checkout's
`doc/validation/2026-09-15-assistant-context/review.md` records the signal recipe,
calculated results and capture hashes. These are separate from the released
0.2.7 screenshots and do not describe the published 0.2.8 packages.

## Scientific citations

Use the [citation guide](/docs/science/references/) for algorithm references.
Cite your measurement, structure source, FEFF backend and software version.
