---
title: "Frequently asked questions"
description: "Short answers about licensing, scope, compatibility, data and support."
audience: user
---

## About rexafs

**What is rexafs?**
rexafs is an open-source toolkit for X-ray absorption spectroscopy. It processes
measured spectra, extracts EXAFS, computes Fourier transforms and fits
scattering-path models. One Rust numerical engine powers a desktop application
and Python, TypeScript and Rust libraries. The [overview](/docs/getting-started/)
lists the features of each interface.

**Is it free? Can I use it at a company or beamline?**
Yes. The project is licensed under MIT or Apache-2.0, at your option, with no
usage restrictions beyond those licenses. Bundled calculation engines,
dependencies and example data keep their own notices; see
[licenses and example data](/licenses/).

**What does the name mean?**
The `r` stands for Rust, the language of the engine, and for reinventing the
wheel for EXAFS analysis. The project began with the need to process large
in-situ measurement series quickly and reproducibly.

**Is the software finished?**
The desktop is stable on macOS and available as previews on Windows and Linux.
Numerical defaults and the project format are documented and covered by
regression tests for every release. The manual states which release it
describes and labels unreleased additions separately. See
[release history](/releases/).

## Scope

**Do I need to program to use rexafs?**
No. The desktop covers import, processing, structures, fitting, series and
publication without code. Start with [your first
analysis](/docs/getting-started/first-analysis/).

**What do the Python and TypeScript packages include?**
Spectrum processing: edge finding, normalization, AUTOBK, forward and inverse
Fourier transforms, with configurable settings and array results. They do not
yet expose groups, data treatment, LCF, PCA, structures or fitting. Those are
available in the desktop and the Rust crate. See the [feature
map](/docs/getting-started/#features-by-interface).

**Can I use it for XANES?**
rexafs normalizes and flattens XANES, overlays spectra, and offers linear
combination fitting and principal component analysis in the desktop and Rust.
It does not perform XANES multiple-scattering simulations or edge fitting.

**Which scattering engine is used?**
ReFEFF is embedded in every desktop package and available as a Rust feature.
The macOS and Linux packages also bundle FEFF10; Windows uses ReFEFF. Record the engine
and version you used; the analysis export includes them.

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
No. Processing and fitting run locally. The only network features are the
online structure databases, the update check and the optional assistant, each
of which you invoke explicitly. The [assistant guide](/docs/desktop/assistant/)
lists exactly what it shares when you use it.

**How do I share an analysis with a colleague?**
Save the project with **Raw: embedded** so original spectra and FEFF inputs are
included, then send the `.rxs` file. For a paper, **Export analysis folder**
writes figures, tables, processed data, a methods draft and references. See
[projects](/docs/desktop/projects/) and [publication](/docs/desktop/publication/).

**Can I recover a project after a bad save or a crash?**
The previous completed save is kept as `.rxs.bak` beside the project. Copy it to
a new `.rxs` name and open it. Saving is atomic, so a failed write leaves the
previous file intact. See [troubleshooting](/docs/troubleshooting/#recover-a-project).

## Results

**Why is the peak in χ(R) not at the bond distance?**
The Fourier transform of χ(k) includes a scattering phase that shifts peaks to
lower R, typically by a few tenths of an ångström. rexafs does not phase-correct
plots. Fitted distances come from the path model, which includes the phase.
See [processing](/docs/science/processing/#4-transform-from-k-to-r).

**Why do my results differ slightly from another program?**
Compare the resolved settings: E₀, normalization ranges, R_bkg, k weight,
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
Check [troubleshooting](/docs/troubleshooting/), then open a
[GitHub issue](https://github.com/Ameyanagi/rexafs/issues) with the rexafs
version, operating system, interface, exact error and a small reproducible
example you are able to share.

**How do I get updates?**
The desktop checks GitHub for releases on startup, without installing anything
automatically. Libraries follow the usual package managers. See [updates and
offline use](/docs/getting-started/updates/).

**Can I contribute?**
Yes. The repository's contributing guide describes the documentation and
scientific standards every change must meet. Issues and pull requests are
welcome on [GitHub](https://github.com/Ameyanagi/rexafs).
