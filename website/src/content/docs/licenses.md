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

The desktop guides show full, unedited window captures from the published
macOS ARM64 0.2.4 package, captured through computer use on 13 September 2026
with the Cu example and built-in Cu structure. The fitting
walkthrough uses ReFEFF, an 8 Å cluster and one first-shell path. Values in these
screenshots describe that demonstration, not a benchmark or universal fit result.

## Scientific citations

Use the [citation guide](/docs/science/references/) for algorithm references.
Cite your measurement, structure source, FEFF backend and software version.
