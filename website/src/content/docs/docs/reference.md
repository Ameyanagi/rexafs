---
title: "API reference and versions"
description: "Generated signatures and source documentation for stable and upcoming APIs."
audience: user
---

The tutorials and stable reference target **published 0.2.4**. Choose Next only
when working from the source checkout.

## Choose your language

These links describe **stable 0.2.4**:

| Task | Python | TypeScript | Rust |
|---|---|---|---|
| Create and process a spectrum | [Spectrum](/docs/reference/stable/python/spectrum/) | [Spectrum](/docs/reference/stable/typescript/spectrum/) | [Spectrum](/api/rust/rexafs/xafs/xasspectrum/struct.XASSpectrum.html) |
| Configure normalization | [PrePostEdge](/docs/reference/stable/python/prepostedge/) | [PrePostEdge](/docs/reference/stable/typescript/prepostedge/) | [PrePostEdge](/api/rust/rexafs/xafs/normalization/struct.PrePostEdge.html) |
| Configure background removal | [AUTOBK](/docs/reference/stable/python/autobk/) | [AUTOBK](/docs/reference/stable/typescript/autobk/) | [AUTOBK](/api/rust/rexafs/xafs/background/struct.AUTOBK.html) |
| Configure the k → R transform | [XrayFFTF](/docs/reference/stable/python/xrayfftf/) | [XrayFFTF](/docs/reference/stable/typescript/xrayfftf/) | [XrayFFTF](/api/rust/rexafs/xafs/xrayfft/struct.XrayFFTF.html) |
| Read files or initialize the runtime | [QAS reader](/docs/reference/stable/python/io-read_qas_transmission/) | [Browser init](/docs/reference/stable/typescript/init-browser/) · [Node init](/docs/reference/stable/typescript/init-node/) | [I/O](/api/rust/rexafs/xafs/io/index.html) |

The [full Rust reference](/api/rust/rexafs/index.html) also covers groups, data
treatment, analysis, structures, fitting and optional plotting.

## Next API · unreleased

[Python](/docs/reference/next/python/spectrum/) and
[TypeScript](/docs/reference/next/typescript/spectrum/) add keyword/options
constructors, direct settings assignment, expanded editor help, and inverse
configuration with `XrayFFTR` and `set_ifft()`.

The [Next Rust API](/api/rust-next/rexafs/index.html) also accepts `AUTOBK` and
`PrePostEdge` directly in setters. Existing enum and `Some(...)` calls remain
supported, and `None` restores defaults.

These signatures are **not in 0.2.4**. Use a source checkout if you need them now;
see the repository's [Python](https://github.com/Ameyanagi/rexafs/tree/main/py-rexafs#build-from-source)
and [TypeScript](https://github.com/Ameyanagi/rexafs/tree/main/js-rexafs#build-from-source)
source-install instructions. The Next API pages are excluded from default site search.

## Reference sources

Stable Python/TypeScript signatures come from release declarations. Their updated
explanations come from maintained docstrings and JSDoc checked against that
release; each page links both sources. These website corrections do not update
the editor help in an installed 0.2.4 package.

Rust references use rustdoc with the default numerical backend and optional
plotting, structure and engine features. They exclude the legacy
`ndarray-compat` backend, which changes parts of the API and their defaults.
Stable uses the published crate; Next uses the checkout.

For equations, units and citations, see [processing theory](/docs/science/processing/)
and [scientific references](/docs/science/references/).
