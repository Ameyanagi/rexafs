# rexafs 0.2.17

Release preparation for keeping FEFF calculations with linked projects and
recovering fits whose FEFF files are missing. Publication requires the final
tagged build, signed Mac artifacts and package verification described in the
[qualification record](validation/2026-11-02-release-0.2.17/review.md).

## FEFF calculations travel with linked projects

FEFF (the scattering-path calculation that supplies the theoretical paths for
an EXAFS fit) writes its path files into rexafs's private folder,
`~/.rexafs/feff`. A project saved with **Link source files** recorded only
where those files were, so after that folder was cleaned, or on another
computer, the project reopened with paths whose files were missing.

Linked projects now embed the FEFF calculation: the path files, `feff.inp`, and
the crystal and engine records. Spectra stay linked, as before. When the
project is opened, FEFF files that are still at their recorded location with
unchanged contents are used; a calculation folder with any file missing or
changed is restored from the embedded copy. Saving again keeps the original
locations.

**Compatibility:** rexafs 0.2.16 and older reject a linked project that
contains embedded data, so they cannot open linked projects saved by 0.2.17.
Projects saved with **Embed source files** are unchanged.

## Recovering missing paths

Projects saved by earlier versions can still reopen without their FEFF files.
Fit › Paths now handles this case:

- A calculation (source) whose FEFF files cannot be read is marked **FEFF
  files missing**, and **Run fit** is disabled with a message naming the
  problem, in both single and joint fits.
- **Recalculate** runs that source's own `feff.inp` again when it is available;
  otherwise the app opens Structure so you can choose the structure and
  calculate again.
- After a calculation, a missing source is re-linked in place when the new
  calculation has a file of the same name for every missing path and any
  geometry recorded in fit history (half path length R_eff, degeneracy and
  number of legs) agrees. The paths keep their parameter expressions and fit
  variables, and joint-fit assignments follow. Otherwise the result is added as
  a new source, as before.
- **×** on a source removes it and its paths after confirmation, together with
  every variable that the remaining model no longer uses.

Previously each recalculation added another source, the missing paths stayed
selected and nothing could be removed, so the fit could not run.

## Compatibility and platforms

Project format remains 1. The 0.2.17 writer adds linked and embedded
compatibility fixtures. The embedded fixture differs from the 0.2.16 one only
in the recorded version and save time; the linked fixture also embeds its FEFF
path file. Historical fixture bytes and data attribution remain intact.

Platform policy is unchanged: macOS desktop and Python distributions require
Apple Silicon. Windows and Linux desktop packages remain previews. The release
qualifies three Python ABI3 wheels on CPython 3.10–3.14 and keeps the Rust,
Python, npm/WebAssembly and desktop versions coordinated.
