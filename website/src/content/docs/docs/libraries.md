---
title: "Choose a library"
description: "Use the same spectrum workflow in Python, TypeScript and Rust."
audience: user
---

Choose the interface that fits your environment. All use the Rust numerical engine.
These quick starts target **0.2.4**, the published package release.

| Library | Best for | Install |
|---|---|---|
| [Python](/docs/libraries/python/) | NumPy, Jupyter and analysis scripts | `python -m pip install rexafs==0.2.4` |
| [TypeScript / JavaScript](/docs/libraries/typescript/) | Node applications and browser tools | `npm install rexafs@0.2.4` |
| [Rust](/docs/libraries/rust/) | Native applications, parallel groups, structures and fitting | `cargo add rexafs@0.2.4` |

Each starts with energy/absorption arrays and a mutable `Spectrum`. Request only
the stage you need: `normalize()`, `calc_background()`, `fft()` or `ifft()`.
Missing prerequisites run automatically.

Python returns NumPy copies; TypeScript returns `Float64Array` copies. Rust getters
borrow slices. The TypeScript/Wasm objects own native memory: release them with
`free()` when finished. All interfaces reject malformed input rather than guessing
its measurement meaning.

The [feature map](/docs/getting-started/#features-by-interface) identifies operations
that are currently desktop/Rust-only. See the [generated API reference](/docs/reference/)
for signatures, fields, defaults and docstrings, including separately labeled
unreleased additions.
