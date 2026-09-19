---
title: "Frequently asked questions"
description: "Short answers about licensing, scope, compatibility, data and support."
audience: user
---

## About rexafs

**What is rexafs?**
An open-source toolkit for processing and fitting X-ray absorption spectra,
with a desktop application and Python, TypeScript and Rust libraries. See the
[feature map](/docs/getting-started/#features-by-interface).

**Is it free? Can I use it at a company or beamline?**
Yes, under your choice of MIT or Apache-2.0. Bundled engines, dependencies and
example data keep their own notices; see
[licenses and example data](/licenses/).

**What does the name mean?**
The `r` stands for Rust, the language of the engine, and for reinventing the
wheel for EXAFS analysis. The project began with the need to process large
in-situ measurement series quickly and reproducibly.

**Is the software finished?**
The desktop is stable on macOS, with Windows and Linux previews. This manual
covers 0.2.11; **Next** documents the source checkout. See
[release history](/releases/).

## Scope

**Do I need to program to use rexafs?**
No. Start with the desktop guide to [your first
analysis](/docs/getting-started/first-analysis/).

**What do the Python and TypeScript packages include?**
Edge finding, normalization including MBACK, AUTOBK, forward/inverse Fourier
transforms, Cauchy wavelets, scalar measurements, XANES peak fits and fluorescence
correction. Groups, broader data treatment, LCF/PCA/MCR, structures and EXAFS fitting are available
in the desktop and Rust only. See the [feature
map](/docs/getting-started/#features-by-interface).

**Can I use it for XANES?**
rexafs normalizes and flattens XANES, overlays spectra, and offers linear
combination fitting, principal component analysis and MCR-ALS in the desktop
and Rust. Since 0.2.10, the desktop and all three libraries fit composite
XANES peaks, steps and baselines. These fits do not simulate XANES multiple
scattering or identify chemical species. The separate [ReFEFF browser preview](/app/scattering/) accepts FEFF
inputs; its current browser validation covers the bundled ZnSe EXAFS workflow.

**Are there Windows or Linux ARM64 downloads?**
Yes. Version 0.2.5 adds ARM64 desktop previews for Windows and Linux alongside
the x64 packages. Windows ARM64 requires Windows 11 for its bundled x64 FEFF10
helper. See [ARM64 availability](/docs/getting-started/install/#arm64-availability).

**Which scattering engine is used?**
Every 0.2.11 desktop package includes ReFEFF and FEFF10. Rust exposes them as
optional features. Record the engine and version you used; the analysis export
includes them. The browser scattering preview uses ReFEFF 0.4.0 separately.

**How does rexafs relate to Athena, Artemis and XrayLarch?**
It is an independent implementation with its own engine, defaults and project
format. Processing results are checked against XrayLarch as a regression
reference, and the manual links to Larch's documentation for background
reading. Defaults differ in places, notably the [AUTOBK endpoint
penalty](/docs/science/autobk/) and the optional [Larch FFT
grid](/docs/science/fourier-compatibility/). The Rust `io` module can read
Athena `.prj` files for interchange.

## Data and files

**Which file formats can I open?**
Plain-text column files from most beamlines, with an import review for column
roles and energy units, and [XDI](/docs/desktop/xdi/) files with named
columns. Structures come from built-in models, CIF or XYZ files, or Materials
Project, AMCSD and COD. See the [glossary of file
formats](/docs/concepts/#file-formats).

**My energy axis is in keV or in monochromator degrees. What do I do?**
Choose the unit in the import review. Angles are converted with Bragg's law
using the monochromator spacing you enter. Do not relabel degrees as eV. See
[import and groups](/docs/desktop/import/).

**Is my data uploaded anywhere?**
Processing and fitting run locally. Startup update checks are enabled by
default; they contact GitHub without uploading spectra. Turn them off in
**Help → Updates**. Online structure searches and the optional assistant also
use the network when you use those features; the [assistant
guide](/docs/desktop/assistant/) explains what it shares. See [updates and
offline use](/docs/getting-started/updates/).

Browser spectrum and scattering inputs also stay local. The website's Cloudflare
hosting collects [page-performance metrics](https://developers.cloudflare.com/web-analytics/data-metrics/data-origin-and-collection/).

**How do I share an analysis with a colleague?**
Save the project with **Raw: embedded** so original spectra and FEFF inputs are
included, then send the `.rxs` file. For a paper, **Export analysis folder**
writes figures, tables, processed data, a methods draft and references. See
[projects](/docs/desktop/projects/) and [publication](/docs/desktop/publication/).

**Can I recover a project after a bad save or a crash?**
Copy the `.rxs.bak` file beside your project to a new `.rxs` filename and open it.
See [project recovery](/docs/troubleshooting/#recover-a-project).

## Results

**Why is the peak in $\chi(R)$ not at the bond distance?**
The Fourier transform of $\chi(k)$ includes a scattering phase that shifts peaks to
lower R, typically by a few tenths of an ångström. rexafs does not phase-correct
plots. Fitted distances come from the path model, which includes the phase.
See [processing](/docs/science/processing/#4-transform-from-k-to-r).

**Why do my results differ slightly from another program?**
Compare the resolved settings: $E_0$, normalization ranges, $R_{\mathrm{bkg}}$, k weight,
window type and range, FFT length and the endpoint penalty. The
[compatibility guide](/docs/science/fourier-compatibility/) lists the known
differences and how to switch to the Larch grid when needed.

**What do the reported uncertainties mean?**
They are standard errors from the local covariance at the solution, scaled by
the noise estimate in use. They describe the fit, not the complete experimental
uncertainty, and correlated parameters can be individually poorly determined.
Read [fit statistics](/docs/science/fitting-statistics/) before quoting them.

## Support

**How do I cite rexafs?**
Cite the version and interface used, the scattering engine, and the primary
papers for the algorithms. The [references page](/docs/science/references/)
lists them, and the analysis export writes `references.md` and
`references.bib`. There is no journal paper for rexafs itself to cite at present.

**Where do I report a problem or ask a question?**
Check [troubleshooting](/docs/troubleshooting/), then [report the problem with a
reproducible example](/docs/troubleshooting/#ask-for-help).

**How do I get updates?**
Use **Help → Updates** in the desktop, or your library package manager. See
[updates and offline use](/docs/getting-started/updates/).

**Can I contribute?**
Yes. Read [CONTRIBUTING.md](https://github.com/Ameyanagi/rexafs/blob/main/CONTRIBUTING.md)
before submitting a pull request.
