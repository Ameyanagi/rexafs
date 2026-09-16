---
title: "API reference and versions"
description: "Generated signatures and source documentation for stable and upcoming APIs."
audience: user
---

The tutorials and stable reference target **published 0.2.9**. Choose Next only
when working from the source checkout.

## Choose your language

These links describe **stable 0.2.9**:

| Task | Python | TypeScript | Rust |
|---|---|---|---|
| Create and process a spectrum | [Spectrum](/docs/reference/stable/python/spectrum/) | [Spectrum](/docs/reference/stable/typescript/spectrum/) | [Spectrum](/api/rust/rexafs/xafs/xasspectrum/struct.XASSpectrum.html) |
| Configure normalization | [PrePostEdge](/docs/reference/stable/python/prepostedge/) | [PrePostEdge](/docs/reference/stable/typescript/prepostedge/) | [PrePostEdge](/api/rust/rexafs/xafs/normalization/struct.PrePostEdge.html) |
| Configure background removal | [AUTOBK](/docs/reference/stable/python/autobk/) | [AUTOBK](/docs/reference/stable/typescript/autobk/) | [AUTOBK](/api/rust/rexafs/xafs/background/struct.AUTOBK.html) |
| Configure the k → R transform | [XrayFFTF](/docs/reference/stable/python/xrayfftf/) | [XrayFFTF](/docs/reference/stable/typescript/xrayfftf/) | [XrayFFTF](/api/rust/rexafs/xafs/xrayfft/struct.XrayFFTF.html) |
| Configure the R → q back-transform | [XrayFFTR](/docs/reference/stable/python/xrayfftr/) | [XrayFFTR](/docs/reference/stable/typescript/xrayfftr/) | [XrayFFTR](/api/rust/rexafs/xafs/xrayfft/struct.XrayFFTR.html) |
| Read a measurement | [read_measurement](/docs/reference/stable/python/io-read_measurement/) | [read_measurement](/docs/reference/stable/typescript/read_measurement/) | [I/O](/api/rust/rexafs/xafs/io/index.html) |
| Initialize WebAssembly | — | [Browser init](/docs/reference/stable/typescript/init-browser/) · [Node init](/docs/reference/stable/typescript/init-node/) | — |

The [full Rust reference](/api/rust/rexafs/index.html) also covers groups, data
treatment, analysis, structures, fitting and optional plotting.

<span id="next-api--unreleased"></span>

## Next API · source checkout

[Python](/docs/reference/next/python/spectrum/) and
[TypeScript](/docs/reference/next/typescript/spectrum/) Next pages and the
[Next Rust reference](/api/rust-next/rexafs/index.html) describe the website's
source checkout. They may include changes after the published release. See the
repository's [Python](https://github.com/Ameyanagi/rexafs/tree/main/py-rexafs#build-from-source)
and [TypeScript](https://github.com/Ameyanagi/rexafs/tree/main/js-rexafs#build-from-source)
source-install instructions. Next pages are excluded from default site search.

## Reference sources

Stable Python/TypeScript signatures come from release declarations. Their updated
explanations come from maintained docstrings and JSDoc checked against that
release; each page links both sources. Website corrections do not update
the editor help in packages already installed.

Rust references use rustdoc with the default numerical backend and optional
plotting, structure and engine features. They exclude the legacy
`ndarray-compat` backend, which changes parts of the API and their defaults.
Stable uses the published crate; Next uses the checkout.

For equations, units and citations, see [processing theory](/docs/science/processing/)
and [scientific references](/docs/science/references/).

The [measurement reader guide](/docs/reference/stable/measurement-reading/) explains automatic beamline, HDF5 and XTUNES import across the libraries and interfaces.
