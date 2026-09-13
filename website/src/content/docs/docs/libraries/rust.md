---
title: "Rust"
description: "Use the full rexafs processing, analysis, structure and fitting API."
audience: user
---

## Install and process

Add the stable crate:

```sh
cargo add rexafs@0.2.4
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

The [generated Rust documentation](/api/rust/rexafs/index.html) is built from the
published 0.2.4 crate with the default nalgebra backend and optional plotting,
solver, calculation-backend and structure-source features. It includes API
comments, types, methods and module explanations. The mutually alternative
`ndarray-compat` backend is excluded so its historical defaults do not replace
the primary implementation in this reference. You can also use
[versioned docs.rs](https://docs.rs/rexafs/0.2.4/rexafs/).

| Module | Operations |
|---|---|
| `Spectrum`, `Group` | Normalization, background, FFT/IFFT, collections and parallel processing |
| `tools` | Calibration, alignment, deglitching, truncation, smoothing, rebinning, merging and differences |
| `analysis` | Bounded linear combination fitting, combination searches, PCA and target transformation |
| `io` | Text/QAS/XDI readers and conversion to spectra |
| `structure` | CIF/XYZ models, symmetry, clusters, neighbors, structure databases and FEFF input |
| `fitting` | FEFF paths, variables/expressions, k/R/q transforms, joint and independent fitting |
| `plot` | Optional spectrum/group/fit figures |

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

For example, `cargo add rexafs@0.2.4 --features plotting,refeff-runner` enables
plotting and the ReFEFF backend. Features describe library capabilities; the
generated reference does not mean every backend is available on every OS.

Follow [theory and units](/docs/science/processing/),
[structural fitting](/docs/desktop/structures/) and
[statistics](/docs/science/fitting-statistics/) when designing an analysis.


## Configure a stage

In stable 0.2.4, configure AUTOBK through the method selector:

```rust
use rexafs::{AUTOBK, BackgroundMethod, Spectrum};

fn configure(spectrum: &mut Spectrum) -> rexafs::Result<()> {
    let mut background = AUTOBK::new();
    background.rbkg = Some(1.2); // Background cutoff in angstroms.
    spectrum.set_background_method(Some(BackgroundMethod::AUTOBK(background)))?;
    Ok(())
}
```

The next release accepts `spectrum.set_background_method(background)?` directly,
and `set_normalization_method(prepost)?` for `PrePostEdge`. Existing enum forms
and `None` for default settings remain supported. Forward and inverse settings
already use `set_fft(transform)` and `set_ifft(inverse)` directly. Follow the
signature shown for your installed version in the API reference.

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
