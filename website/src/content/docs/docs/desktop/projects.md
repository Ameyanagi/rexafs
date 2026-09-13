---
title: "Projects, sharing and recovery"
description: "Save portable inputs and recover the previous project save."
audience: user
---

**`.rxs` is the only project suffix.** Format 1 is the first release format.
Unreleased codename formats have no compatibility loader or file association.
Use **Save project** and **Open project** in the desktop app.

Projects store processing settings, overrides, derived spectra, fit models/history,
joint assignments and publication settings, including figure sizes and captions.
Materials Project credentials remain in machine-local settings.

## Raw data: paths or embedded originals

Choose **Raw: paths / Raw: embedded** beside **Save project**. This choice is
remembered in the project and also applies to **Export analysis folder**.

| Mode | Contents | Moving or sharing |
|---|---|---|
| **Paths (default)** | Source references and metadata; no original file payloads | Move the project and source folders together, preserving their relative layout |
| **Embedded** | Losslessly compressed original spectra and referenced FEFF inputs | Reopen the `.rxs` file without its original input files |

Relative paths resolve against the **directory containing the project**, never the
process working directory. For example, `run.rxs` beside a `data` folder stores
`data/cu.xmu`; a project inside `projects` stores `../data/cu.xmu`. Save As
recalculates references to the same files. Different Windows drives require
absolute paths. The save dialog starts in the current project's directory, or
the source folder for a new project.

Embedded mode includes the imported spectrum folder, standalone sources, override
and joint-fit inputs, referenced current/historical FEFF paths, and available
`feff.inp`, `crystal.json` and `engine.txt` workspace metadata. It waits for
catalog scanning and fails explicitly if a required source is missing or unreadable.
Original bytes, including comments, line endings and non-text bytes, survive
unchanged. Identical files share one compressed payload. Extraction checks byte
counts and SHA-256 hashes and writes only inside the private rexafs cache.

Paths mode permits recording unavailable sources so settings can still be saved;
unavailable entries lack size/checksum metadata. Linked files can change independently.
Their recorded hashes identify the bytes present at save time. Use embedded mode
for a portable input snapshot. Switching an opened embedded project to paths
references its currently extracted cache files; keep embedded mode for sharing it.

Processed spectra are recomputed when reopening. Derived spectra created inside
the app retain full energy and μ arrays in both modes. Historical fit statistics
and models remain saved; historical curve arrays are available through analysis
exports when present in the session. Embedded mode does not include unrelated
workspace files or a saved executable/backend. **Export analysis folder** adds
processed arrays, available fit arrays and captioned figures/tables beside `project.rxs`.

## File information and limits

Projects record the software/format version, source references, timestamps and
checksums. Original comment headers are retained within a bounded preview, while
embedded payloads preserve the complete original bytes. Project files are limited
to 512 MiB and expanded inputs to 1 GiB. Missing/corrupt embedded payloads and
unsafe archive paths produce errors.

## Compatibility from the first release onward

- Application and project-format versions are independent. Optional additions can
  retain format 1 only if their interpretation stays compatible. Existing defaults
  are part of the format contract.
- Loading never rewrites the source. Saved values, bounds, expressions, derived
  arrays and assignments must survive load/save/reopen. Unknown top-level metadata
  is retained; undocumented nested extension fields are outside this contract.
- Future formats are rejected with an actionable error; failed loading leaves the
  session intact. Saving over a future-format file is refused. Missing versions,
  missing headers and malformed input are errors.
- Incompatible formats require an explicit migration; an unsupported newer format
  produces an error rather than being interpreted as an older project.

Backward compatibility means **new releases read projects from earlier released
versions**. An older executable cannot be guaranteed to retain later features
when saving a newer file. Keep the original/backup when switching versions.

## Safe saving and recovery

The complete project is prepared and validated before the destination changes.
A temporary file in the same directory is written, flushed and renamed into place.
Failed writes, syncs and renames leave the previous project intact. Replacing a
project first retains its exact previous bytes in `project.rxs.bak`; only the
immediately preceding save is retained.

To recover, copy `project.rxs.bak` to `recovered.rxs` and open the copy. This
backup is on the same disk; keep separate archival copies of valuable experiments.



[![Full save-project dialog with embedded source files selected](/screenshots/save-project.jpg)](/screenshots/save-project.jpg)

*Choose links for a shared folder layout or included source files for portability. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*
