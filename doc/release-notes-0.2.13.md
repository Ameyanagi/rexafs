# rexafs 0.2.13

Release preparation. Version 0.2.12 remains the latest published stable release
until the exact 0.2.13 tag has passed build, signing, installation and publication
checks. The [qualification record](validation/2026-09-22-release-0.2.13/review.md)
records the source and results as each step completes.

## A reproducible copper teaching example

**Synthetic copper reduction…**, available from the start screen and Help menu,
opens three measured CuO, Cu₂O and Cu references plus 50 synthetic spectra. The
sequence starts with CuO, passes through a Cu₂O-rich region and ends with Cu.
Every frame is a deterministic linear combination of the original raw absorption
μ(E), before normalization. No random fractions or additional noise are added.

Group names show the known mixing fractions; project metadata retains their full
precision and the measured references' provenance. The project includes all its
input data and opens with ordinary automatic processing settings. Only the
50 mixtures are initially marked, leaving the references available for an
independent linear-combination fit (LCF).

The [tutorial](synthetic-copper-reduction.md) walks through principal component
analysis (PCA), blind multivariate curve resolution by alternating least squares
(MCR-ALS), known-reference LCF and Series differences. Retained numerical results
and reproduction commands accompany SVG, vector PDF and 600 dpi PNG figures.
The figures use ruviz's default styling with smaller comparison markers.

For the retained automatic-processing example, three PCA components account for
99.9998568% of the uncentered squared signal. After the documented approximate
reference edge-step conversion, mean raw-weight errors are 1.051 percentage
points for blind MCR and 0.047 points for known-reference LCF. The conversion is
performed by the validation helper, not the desktop Fractions view. These are
results for a teaching dataset, not experimental concentration error guarantees.

## Analysis defaults

PCA plots start on a linear scale; logarithmic display remains available. MCR-ALS
has its own energy-range controls. Its **Auto** range uses the full common energy
interval of the selected spectra rather than borrowing the PCA XANES range.
Users can still enter a narrower interval explicitly. The bundled example does
not apply the separate validation metadata as processing overrides.

## Storage and installer cleanup

**Help → Storage**, **About rexafs → Storage**, and the command palette open the
new [storage view](desktop-storage.md). It scans managed installer downloads and
inactive updater app copies in the background, shows their combined logical size,
and offers buttons to open the relevant hidden folders.

**Delete listed files** removes only the reviewed candidates after checking that
they have not changed. Download and installation locks coordinate with cleanup.
Current applications, recovery projects, saved scientific data, RMC checkpoints,
FEFF calculations, settings and credentials remain intact. On macOS, deleting an
inactive previous-app copy removes that transaction's rollback copy. The view
does not search the user's Downloads folder or remove manually installed apps.

## Dependencies, support and compatibility

This release includes the reviewed hdf5-pure, ureq, rand and rand_chacha updates.
The RNG migration retains the project's historical random-number streams and
saved-state compatibility; the [dependency record](dependencies.md) and
[retained-stream test](../crates/rexafs/tests/rmc_rng_compatibility.rs) explain
and verify the migration.
The README and manual also link to GitHub sponsorship and repository stars.

Project format remains 1. New linked and embedded compatibility projects are
written through the 0.2.13 writer; earlier fixtures and the teaching dataset's
original bytes and attribution are retained.

Platform policy is unchanged: macOS desktop and Python distributions require
Apple Silicon. Windows and Linux desktop packages remain previews. Python uses
three ABI3 wheels, qualified on CPython 3.10–3.14; Rust, npm/WebAssembly and desktop
versions remain coordinated.
