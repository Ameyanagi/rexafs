---
title: "Rust"
description: "Use the full rexafs processing, analysis, structure and fitting API."
audience: user
---

## Install and process

Add the stable crate:

```sh
cargo add rexafs@0.2.9
```

```rust
use rexafs::Spectrum;

fn transform(energy: &[f64], mu: &[f64]) -> rexafs::Result<()> {
    let mut spectrum = Spectrum::from_arrays(energy, mu)?;
    spectrum.fft()?;
    println!("{:?}", spectrum.chir_mag());
    Ok(())
}
```

Energy is in eV. Inputs must be finite and energy strictly increasing. Rust methods
return `Result`. `k()` and `chi()` borrow slices; `chir()` borrows the complete
complex FFT representation. Other array getters return owned vectors. Getters
return `None` when results are unavailable and never calculate a stage implicitly.

Rust setters take ownership of configurations. Clone a setting explicitly to reuse
it. Changed inputs/settings invalidate dependent results through the setters;
after direct edits to legacy public fields, call `invalidate_derived()`.

## Full API reference

The [stable reference](/api/rust/rexafs/index.html) uses the published 0.2.9 crate
with the default nalgebra backend and optional features. It excludes the legacy
`ndarray-compat` backend, which replaces parts of that API and has different
defaults. [Versioned docs.rs](https://docs.rs/rexafs/0.2.9/rexafs/) is also available.

The [Next reference](/api/rust-next/rexafs/index.html) uses the checkout with the
same features. Use it when working from source.

| Module | Operations |
|---|---|
| `Spectrum`, `Group` | Normalization, background, FFT/IFFT, collections and parallel processing |
| `tools` | Calibration, alignment, deglitching, truncation, smoothing, rebinning, merging and differences |
| `analysis` | Bounded linear combination fitting, combination searches, principal component analysis and target transformation |
| `io` | Content-detected measurement import, raw columns/datasets and explicit signal selection |
| `structure` | CIF/XYZ models, symmetry, clusters, neighbors, structure databases and FEFF input |
| `fitting` | FEFF paths, variables/expressions, k/R/q transforms, joint and independent fitting |
| `plot` | Optional spectrum/group/fit figures |

Read beamline files with `rexafs::io::read_measurement(path)`. Inspect the returned
scans and select stored absorption, transmission or fluorescence by column name
or index. The [reader guide](/docs/reference/stable/measurement-reading/) explains
automatic choices, units, angle conversion and supported containers.

## Optional features

| Cargo feature | Enables |
|---|---|
| `trust-region` | Default trust-region solver support |
| `plotting` | ruviz plotting integration |
| `refeff-runner` | Embedded ReFEFF calculations |
| `feff10-runner` | FEFF10 integration; verify platform compatibility |
| `amcsd` | Local AMCSD catalog support |
| `materials-project`, `cod` | HTTP structure-source integrations |
| `ndarray-compat` | Legacy ndarray compatibility backend |

For example, `cargo add rexafs@0.2.9 --features plotting,refeff-runner` enables
plotting and ReFEFF. Backend availability depends on the platform; see
[WebAssembly support](/docs/libraries/webassembly/) for browser limits.

Follow [theory and units](/docs/science/processing/),
[structural fitting](/docs/desktop/structures/) and
[statistics](/docs/science/fitting-statistics/) when designing an analysis.

## Configure a stage

Pass settings directly to the stage setter:

```rust
use rexafs::{AUTOBK, Spectrum};

fn configure(spectrum: &mut Spectrum) -> rexafs::Result<()> {
    let mut background = AUTOBK::new();
    background.rbkg = Some(1.2); // Background cutoff in angstroms.
    spectrum.set_background_method(background)?;
    Ok(())
}
```

Normalization similarly accepts `set_normalization_method(prepost)?`.
Existing enum forms, `Some(...)` and `None` for default settings remain supported.
Use `set_fft(transform)` for `XrayFFTF` forward settings and `set_ifft(inverse)`
for `XrayFFTR` inverse settings. Each setter invalidates the affected stage and
its dependents; run `fft()` or `ifft()` to calculate the new results.

## Collections and errors

`Group::fft()` processes all members in parallel with each spectrum's settings;
`fft_seq()` runs sequentially. Both attempt every member and collect indexed
errors. Successful members retain their results if another fails, so a batch is
not an atomic transaction. The same execution choices exist for normalization,
background removal, edge detection and inverse transforms.

Use `group.spectra.get(index)` for ordinary checked access. The legacy
`get_spectrum(index)` method returns the last member for an oversized index and
errors only when the group is empty.

The `io` module also supports Athena `.prj` interchange. Its legacy JSON/BSON
serializers are separate from the desktop `.rxs` project format and do not provide
the desktop's backup, migration or atomic replacement behavior.
