# rexafs 0.2.16

Release preparation for focused Process plots and a still XANES view while E₀
is dragged. Publication requires the final tagged build, signed Mac artifacts
and package verification described in the
[qualification record](validation/2026-10-30-release-0.2.16/review.md).

## Process plot focus

The linked Process plots (measured μ(E), normalized μ(E), kʷχ(k) and |χ(R)|)
showed every step's ranges and fitted curves at once. **Focus** in the plot bar
now limits them to one step:

- **All** shows every range, as before.
- **Normalize** shows the pre-edge and post-edge lines and ranges, and dμ/dE
  when it is on.
- **Background** shows the spline, its k range (on both μ(E) plots and on χ(k))
  and Rbkg.
- **Forward FT** shows the k window on χ(k).
- **Back FT** shows the R range and the back-transform R window on |χ(R)|.

The spectra themselves stay visible in every mode. Focus follows your work:
pressing an inspector section, editing a value, pressing a plot range or
handle, or turning on a curve button (dμ/dE, Pre/post, Spline, Window) selects
the matching step. The Transform section is split into **Forward FT** and
**Back FT** to match.

dμ/dE, the first derivative of μ(E) used to locate the edge, is now drawn on
the measured μ(E) plot as well as the normalized plot. It is scaled by the edge
step and offset to the pre-edge line so its peak sits within the spectrum.

## Setting E₀ in the XANES view

The **XANES** and **−200…+800** views are positioned relative to E₀, so
dragging the E₀ line used to move the whole view under the cursor. The view now
stays still while E₀ is dragged, so the line moves over a fixed spectrum, and
re-centres on the new E₀ when the mouse is released.

## Dependencies

Rust dependency updates include pyo3 0.29.3, rusqlite 0.41, encoding_rs 0.8.42
and AccessKit fixes for macOS and Windows. Website builds use Starlight 0.42.7,
KaTeX 0.19.1 and updated Playwright test tools. Each update passed the full CI
matrix before merging.

## Compatibility and platforms

Project format remains 1; the Process focus and the XANES view position are not
saved. The 0.2.16 writer adds linked and embedded compatibility fixtures, which
differ from the 0.2.15 pair only in the recorded version and save times.
Historical fixture bytes and data attribution remain intact.

Platform policy is unchanged: macOS desktop and Python distributions require
Apple Silicon. Windows and Linux desktop packages remain previews. The release
qualifies three Python ABI3 wheels on CPython 3.10–3.14 and keeps the Rust,
Python, npm/WebAssembly and desktop versions coordinated.
