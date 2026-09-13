---
title: "API reference and versions"
description: "Generated signatures and source documentation for stable and upcoming APIs."
audience: user
---

The main tutorials use **published 0.2.4**. The reference pages below are generated
from declarations and docstrings so field names and signatures do not depend on a
second handwritten API list.

## Choose your language

Start with **[Python Spectrum](/docs/reference/stable/python/spectrum/)**,
**[TypeScript Spectrum](/docs/reference/stable/typescript/spectrum/)** or
**[Rust crate reference](/api/rust/rexafs/index.html)**.

Use this index to jump directly to the operation or settings you need. These
links describe **stable 0.2.4**.

| Task | Python | TypeScript | Rust |
|---|---|---|---|
| Create and process a spectrum | [Spectrum](/docs/reference/stable/python/spectrum/) | [Spectrum](/docs/reference/stable/typescript/spectrum/) | [Spectrum](/api/rust/rexafs/xafs/xasspectrum/struct.XASSpectrum.html) |
| Configure normalization | [PrePostEdge](/docs/reference/stable/python/prepostedge/) | [PrePostEdge](/docs/reference/stable/typescript/prepostedge/) | [PrePostEdge](/api/rust/rexafs/xafs/normalization/struct.PrePostEdge.html) |
| Configure background removal | [AUTOBK](/docs/reference/stable/python/autobk/) | [AUTOBK](/docs/reference/stable/typescript/autobk/) | [AUTOBK](/api/rust/rexafs/xafs/background/struct.AUTOBK.html) |
| Configure the k → R transform | [XrayFFTF](/docs/reference/stable/python/xrayfftf/) | [XrayFFTF](/docs/reference/stable/typescript/xrayfftf/) | [XrayFFTF](/api/rust/rexafs/xafs/xrayfft/struct.XrayFFTF.html) |
| Read files or initialize the runtime | [QAS reader](/docs/reference/stable/python/io-read_qas_transmission/) | [Browser init](/docs/reference/stable/typescript/init-browser/) · [Node init](/docs/reference/stable/typescript/init-node/) | [I/O](/api/rust/rexafs/xafs/io/index.html) |

## Stable 0.2.4

- [Python Spectrum](/docs/reference/stable/python/spectrum/),
  [normalization settings](/docs/reference/stable/python/prepostedge/),
  [AUTOBK settings](/docs/reference/stable/python/autobk/),
  [forward FFT](/docs/reference/stable/python/xrayfftf/),
  [QAS reader](/docs/reference/stable/python/io-read_qas_transmission/).
- [TypeScript Spectrum](/docs/reference/stable/typescript/spectrum/),
  [normalization settings](/docs/reference/stable/typescript/prepostedge/),
  [AUTOBK settings](/docs/reference/stable/typescript/autobk/),
  [forward FFT](/docs/reference/stable/typescript/xrayfftf/).
- [Full Rust API, default backend](/api/rust/rexafs/index.html), including groups,
  processing, data treatment, analysis, I/O, structures, fitting and plotting.

Python and TypeScript signatures and available members come from the release's
declarations. Their explanations come from the maintained docstrings and JSDoc,
reviewed against the released implementation. This lets documentation corrections
reach stable users while keeping unreleased signatures on the Next pages. Each
page links both sources. Rust pages use rustdoc on the published crate with the
default numerical backend and optional plotting, structure and engine features.
The legacy `ndarray-compat` backend is excluded because it replaces parts of that
API and has different defaults.

## Next API · unreleased

[Python](/docs/reference/next/python/spectrum/) and
[TypeScript](/docs/reference/next/typescript/spectrum/) and
[Rust](/api/rust-next/rexafs/index.html) references describe the
current source additions: documented keyword/options constructors, direct settings
assignment, richer completion/hover help, and inverse configuration with
`XrayFFTR` and `set_ifft()`.

Rust's source API also accepts `AUTOBK` and `PrePostEdge` directly in the settings
setters. The conversion to the method enum happens automatically. Existing enum
and `Some(...)` calls remain supported, and `None` restores default settings.

These signatures are **not in 0.2.4**. Use a source checkout if you need them now;
see the repository's [Python](https://github.com/Ameyanagi/rexafs/tree/main/py-rexafs#build-from-source)
and [TypeScript](https://github.com/Ameyanagi/rexafs/tree/main/js-rexafs#build-from-source)
source-install instructions. The Next API pages are excluded from default site search.

The references preserve citations present in the API help. Read the
[scientific references](/docs/science/references/) for their context, and
[processing theory](/docs/science/processing/) for equations and units.
