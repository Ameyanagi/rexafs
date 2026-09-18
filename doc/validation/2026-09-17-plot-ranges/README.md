# Plot ranges and Transform wavelets — 17 September 2026

Checked with native computer use on macOS ARM64 using an optimized local build
from `feature/analysis-b-f` after `87cbbf3`, together with the local ruviz
`fix/gpui-context-menu-layer` change. This is development evidence, not a release
or native Windows/Linux qualification. Input: the public, CC0 APS 13-ID-C
`cu_metal_rt.xdi` retained in the [experimental reference set](../../../crates/rexafs/tests/fixtures/analysis/experimental-larch/README.md).
No unpublished data were used.

Automated checks: 591 desktop tests passed, 6 ignored; all 90 ruviz GPUI adapter
tests passed; all 8 MBACK/wavelet reference tests passed, including the 6 new
experimental cases. Strict Clippy passed for the new experimental integration
test and core dependency. `cargo package --list` contained 440 entries and no
experimental fixture, original beamline corpus or experimental integration test.
The optimized desktop build completed with 12 existing dead-code warnings.
After extending the toggle to MBACK fit intervals and peak exclusions, the
195 desktop shell tests passed again. The full suite above preceded that final
display-only extension.

The initial checks used the separate local ruviz worktree. The command below is
retained as historical build evidence; it is superseded by the registry dependency
update on 18 September 2026 described below.

```sh
cargo build --release -p rexafs-gui --bin rexafs \
  --config 'patch.crates-io.ruviz.path="/Users/ryuichi/dev/ruviz-menu-layer"' \
  --config 'patch.crates-io.ruviz-gpui.path="/Users/ryuichi/dev/ruviz-menu-layer/adapters/gpui"'
```

These machine-local overrides were used only for the initial validation. The fix
was merged in [ruviz PR #190](https://github.com/Ameyanagi/ruviz/pull/190) and
published in [ruviz 0.14.2](https://github.com/Ameyanagi/ruviz/releases/tag/v0.14.2).
The rexafs workspace now requires ruviz and ruviz-gpui 0.14.2, and the lockfile
resolves both to crates.io packages with registry checksums. Local ruviz
overrides are no longer needed.

The fixed-position range icon in the main toolbar hides or shows selection
shading, lines and handles. It is a window display preference, initially on;
it does not change the processing definitions. The separate Fourier **Window**
button still controls the taper curve.

Initial verified interactions (before the shared-selector refinement below):

- Hiding and showing normalization ranges kept the plot dimensions, viewport,
  button positions and parameter values unchanged.
- Dragging the pre-edge upper handle changed its offset from Auto (−30 eV) to
  −57 eV. Hiding and restoring ranges retained −57 eV.
- Right-click menus painted above the selection shading and boundaries. Clicking
  **Reset View** directly over the normalization boundary activated the menu
  without editing the bound. The same menu check passed over a wavelet rectangle.
- MBACK's Atomic match view hid both fitted intervals while retaining a manually
  panned viewport and the same fitted scale. Restoring the full view showed the
  same curves without shading. Peak-fit exclusions use the same visibility
  preference; that final path was code-reviewed, not separately exercised in
  native computer use.
- **Transform → Wavelet** opened the wavelet workspace; Data no longer listed it.
  A native map of the measured Cu spectrum calculated successfully. This GUI
  check used the native defaults, separate from the matched Larch test settings.
- Dragging the wavelet k lower bound changed it from 4.00 to 4.68 Å⁻¹. Hiding
  the selections removed the map rectangle and both lower-plot ranges, while
  the bound and displayed region integral (1.37228 Å⁻²) stayed unchanged.
- **Save region → ← Fourier → Wavelet → Region 1** restored the saved bounds
  and retained map. The app was left on that experimental Cu wavelet.

The ruviz fix defers context-menu painting until after ordinary host overlays.
Its new `is_context_menu_open()` query lets rexafs leave menu presses alone
instead of capturing them as handle drags. The menu needs its existing plot
event handlers; an occluding menu hitbox was tested and rejected because it
blocked those handlers. No registry source was modified.

| Evidence | Image |
| --- | --- |
| Hidden normalization ranges, edited bound retained | [Screenshot](ranges-hidden.jpg) |
| Menu above visible normalization ranges | [Screenshot](menu-above-ranges.jpg) |
| Experimental Cu wavelet in Transform | [Screenshot](wavelet-transform.jpg) |
| Experimental Cu MBACK without interval shading | [Screenshot](mback-ranges-hidden.jpg) |

## Shared Transform selector follow-up

The standalone Wavelet button and **← Fourier** return button were replaced with
one **k · R · k + R · q · Wavelet** selector. The earlier screenshots and
interaction notes above preserve the intermediate design.

An optimized macOS build with the same local ruviz overrides completed, and all
195 desktop shell tests passed again. Native computer use verified:

- Each of the five views opened with the correct plots and inspector. The
  selector stayed at the same position, with one selected view.
- Selecting **q** exposed the inverse-transform settings.
- In Wavelet, an unsaved lower region bound of 5.10 Å⁻¹, the **Real** display,
  **Slices**, and the displayed integral of 1.28803 Å⁻² survived switching to
  **k + R** and back. The retained map remained available without recalculation.
- **History** opened from the relocated toolbar and was dismissed by clicking
  the button again. The Cu demo was left open in **Wavelet → Magnitude** with
  the original spectra shown beneath the map.

| Evidence | Image |
| --- | --- |
| Shared selector with experimental Cu wavelet | [Screenshot](wavelet-selector.jpg) |
| Unsaved region and display modes retained after switching | [Screenshot](wavelet-selector-retained.jpg) |

## Released ruviz follow-up — 18 September 2026

The workspace manifest and lockfile now resolve both ruviz and ruviz-gpui to
0.14.2 from crates.io. The dependency update changed only those two lockfile
entries. `cargo tree` confirmed a single shared copy of each package. The
published GPUI adapter includes the deferred menu layer and
`RuvizPlot::is_context_menu_open()`.

The following commands passed without local ruviz overrides:

```sh
cargo build --locked --release -p rexafs-gui --bin rexafs
cargo test --locked -p rexafs-gui --bin rexafs
cargo test --locked -p rexafs --features plotting --lib plot::
```

The desktop suite passed 591 tests with 6 ignored; the optional core plotting
suite passed all 23 selected tests. The optimized build retained
the same 12 existing dead-code warnings. Formatting and diff checks also passed.

Native computer use in a separate Cu validation app confirmed that **Reset View**
restored a panned normalization viewport when clicked directly over the hidden
range boundary beneath the menu. The pre-edge end remained −57 eV and the other
range settings stayed unchanged. The wavelet menu also rendered above its region
rectangle and accepted **Reset View** without changing the region. Hiding and
restoring ranges preserved the map and its displayed integral of 1.48686 Å⁻².
The shared Transform selector remained visible in the Wavelet view.

| Evidence from the registry-built app | Image |
| --- | --- |
| Menu above the panned normalization range | [Screenshot](ruviz-0142-menu.jpg) |
| Menu above the wavelet region | [Screenshot](ruviz-0142-wavelet-menu.jpg) |
