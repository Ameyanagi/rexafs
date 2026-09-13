---
title: "API reference and versions"
description: "Generated signatures and source documentation for stable and upcoming APIs."
audience: user
---

The main tutorials use **published 0.2.4**. The reference pages below are generated
from declarations and docstrings so field names and signatures do not depend on a
second handwritten API list.

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
- [Full Rust API, all features](/api/rust/rexafs/index.html), including groups,
  processing, data treatment, analysis, I/O, structures, fitting and plotting.

Python reference signatures come from the release's packaged stubs and its installed
wheel docstrings. TypeScript pages use the release declarations and JSDoc. Rust
pages use rustdoc on the published crate. Empty or brief upstream help is not
silently replaced with an invented API contract; the tutorials and scientific
guides supply the surrounding explanation.

## Next API · unreleased

[Python](/docs/reference/next/python/spectrum/) and
[TypeScript](/docs/reference/next/typescript/spectrum/) references describe the
current source additions: documented keyword/options constructors, direct settings
assignment, richer completion/hover help, and inverse configuration with
`XrayFFTR` and `set_ifft()`.

These signatures are **not in 0.2.4**. Use a source checkout if you need them now;
see the repository's [Python](https://github.com/Ameyanagi/rexafs/tree/main/py-rexafs#build-from-source)
and [TypeScript](https://github.com/Ameyanagi/rexafs/tree/main/js-rexafs#build-from-source)
source-install instructions. The Next API pages are excluded from default site search.

The references preserve citations present in the API help. Read the
[scientific references](/docs/science/references/) for their context, and
[processing theory](/docs/science/processing/) for equations and units.
