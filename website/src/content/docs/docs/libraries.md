---
title: "Choose a library"
description: "Use the same spectrum workflow in Python, TypeScript and Rust."
audience: user
---

Choose the interface that fits your environment. All use the Rust numerical engine.
These quick starts target **0.2.4**, the published package release.

| Library | Best for | Install |
|---|---|---|
| [Python](/docs/libraries/python/) | NumPy, Jupyter and analysis scripts | `uv add rexafs==0.2.4` |
| [TypeScript / JavaScript](/docs/libraries/typescript/) | Bun, Node applications and browser tools | `bun add rexafs@0.2.4` |
| [Rust](/docs/libraries/rust/) | Native applications, parallel groups, structures and fitting | `cargo add rexafs@0.2.4` |

We recommend [uv](https://docs.astral.sh/uv/getting-started/installation/) for Python
projects and [Bun](https://bun.sh/docs/installation) for TypeScript projects. The
table's install commands run inside an existing project. For a new Python analysis:

```sh
uv init --python 3.12 rexafs-analysis
cd rexafs-analysis
uv add rexafs==0.2.4 numpy
uv run python -c "import rexafs; print(rexafs.__version__)"
```

Save the [Python example](/docs/libraries/python/#load-and-transform-a-measured-spectrum) as
`analyze.py` in this directory, then run `uv run analyze.py`. uv manages the
project's `.venv` automatically. Keep `pyproject.toml` and `uv.lock` with your
analysis code so collaborators can use the recorded dependencies. This follows
uv's [project workflow](https://docs.astral.sh/uv/guides/projects/).

For a new TypeScript project, run `bun init` before `bun add rexafs@0.2.4`.
The [TypeScript guide](/docs/libraries/typescript/) includes a runnable example
and editor setup.

Each starts with energy/absorption arrays and a mutable `Spectrum`. Request only
the stage you need: `normalize()`, `calc_background()`, `fft()` or `ifft()`.
Missing prerequisites run automatically.

Python returns NumPy copies; TypeScript returns `Float64Array` copies. Rust getters
borrow stored arrays or allocate derived arrays, depending on the getter; their
return types make ownership explicit. The TypeScript/Wasm objects own native memory: release them with
`free()` when finished. Array constructors validate the supplied numerical arrays;
file readers have their own format and validation rules. Confirm energy units and
the meaning of the absorption columns before processing.

The [feature map](/docs/getting-started/#features-by-interface) identifies operations
that are currently desktop/Rust-only. See the [generated API reference](/docs/reference/)
for signatures, fields, defaults and docstrings, including separately labeled
unreleased additions.
