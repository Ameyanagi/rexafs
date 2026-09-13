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
return `Result`; getters borrow slices when available. Prefer the checked spectrum
setters so changed inputs/settings invalidate dependent results correctly.

## Full API reference

The [generated Rust documentation](/api/rust/rexafs/index.html) is built from the
published 0.2.4 crate with all feature flags and includes its API comments, types,
methods and module explanations. You can also use
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
all-features documentation does not mean every backend is available on every OS.

Follow [theory and units](/docs/science/processing/),
[structural fitting](/docs/desktop/structures/) and
[statistics](/docs/science/fitting-statistics/) when designing an analysis.
