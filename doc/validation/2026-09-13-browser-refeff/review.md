# Browser ReFEFF qualification

This pre-deployment record covers completed local verification of the rexafs
browser integration. ReFEFF 0.4.0 itself is already published. The checks below
used its released WebAssembly bytes on a local preview server; production
verification follows the deployment gate below.

## Engine and reference identities

The browser uses the [ReFEFF 0.4.0 release](https://github.com/Ameyanagi/refeff/releases/tag/v0.4.0)
at source commit `91b23364309c3daca871b748ae45bf2426c60b8e`. Its bundled CLI reports
0.3.0; this is a separate package version. The target is `wasm32-wasip1`, running
serially in a Web Worker with an in-memory filesystem.

| Artifact | SHA-256 |
| --- | --- |
| Published `refeff-wasm-0.4.0.zip` | `22ad5127e25d85d8f02583b686e3b0f58c66a91adc11a15653d1e1668697fafb` |
| Browser `refeff.wasm` (12,694,588 bytes) | `3a08f446b302ac4ca665d6aae738eead15915f79582162ce610e7afad41433cc` |
| Published `release-parity-evidence.zip` | `8232a91e81592c612e1c52e663c88e92ab8076777e051fa2635798fee24f7c07` |
| Native reference executable | `29acb518ae122015cb144d7c4bc546bba8f23c5006a44c2d81c936ad1dac50e6` |
| Unchanged upstream `znse.inp` | `c2b4494b9a39306a8429d432990dd5e354ea87ba6abd999f23cefdccf6045576` |

Before generating reference arrays, the local native executable's SHA-256 was
checked against `provenance.rustBinarySha256` in the published
[parity evidence](https://github.com/Ameyanagi/refeff/releases/download/v0.4.0/release-parity-evidence.zip).
That evidence identifies the same source commit and Rust 1.95.0
(`59807616e`, 2026-04-14). The native executable ran in a fresh temporary
directory with `--threads 1 --json run --input feff.inp --output native`.
Its output was retained before running the browser comparison; browser output
was not used to generate expected values.

The [reference provenance](../../../website/tests/fixtures/refeff-0.4.0/provenance.json)
records the native input, executable and output hashes, generation command,
host, upstream attribution and license link. The frozen
[chi.dat](../../../website/tests/fixtures/refeff-0.4.0/chi.dat) and
[xmu.dat](../../../website/tests/fixtures/refeff-0.4.0/xmu.dat) retain their original
headers. Release asset identities are pinned in
[refeff-release.json](../../../website/src/data/refeff-release.json).

## Scientific comparison

The unmodified [upstream input](https://github.com/Ameyanagi/refeff/blob/91b23364309c3daca871b748ae45bf2426c60b8e/crates/refeff/tests/data/znse.inp)
contains 35 atoms and four potentials. Although historically named ZnSe, it
includes one krypton (Kr) scatterer in the first coordination shell. The browser
labels it as a test input and identifies the Kr substitution. These results
must not be presented as a calculation for pristine ZnSe.

The complete extended X-ray absorption fine structure (EXAFS) calculation
returned a successful versioned CLI report, one effective calculation thread,
full multiple scattering (`fms`) and final spectrum (`ff2x`) stages, and 15
`feffNNNN.dat` scattering-path files. The plot reads the first two columns of
`chi.dat`: photoelectron wave number, k, in Å⁻¹ and dimensionless χ(k), without
additional weighting or normalization.

Every numeric value in both spectra passed the upstream
[native/WASM comparison rule](https://github.com/Ameyanagi/refeff/blob/91b23364309c3daca871b748ae45bf2426c60b8e/wasm/tests/runtime.test.mjs):

```text
abs(browser - native) <= 5e-8 + 5e-5 * abs(native)
```

Here `browser` and `native` are corresponding values in the same output column,
in that column's units. The absolute term is applied in those units; the
relative term is dimensionless. This is an implementation agreement tolerance,
not an experimental uncertainty. Comments and timing text are excluded.

| Output | Numeric rows | Columns | Values compared | Largest fraction of allowed difference |
| --- | ---: | ---: | ---: | ---: |
| `chi.dat` | 401 | 4 | 1,604 | 0.418223 |
| `xmu.dat` | 401 | 6 | 2,406 | 0.418572 |
| Total | | | **4,010** | **All below 1** |

The comparison establishes native/browser agreement for this input and these
outputs. It does not establish physical accuracy, validate every FEFF workflow,
or extend the browser preview to fitting or the complete desktop interface.

## Browser and interface checks

Local verification used Apple Silicon macOS 26.5.1, Node.js 24.19.0,
Playwright 1.63.0 and headless Chromium 153.0.8010.12. The preview served
`127.0.0.1:4321`; the production URL settings below selected paths and canonical
links, not remote calculation services.

- **Seven scattering browser tests passed at `/`**, with
  `SITE_URL=https://rexafs.com SITE_BASE=/`. A repeat after the input label was
  changed to **Load ZnSe test input** also passed; the exported source name
  identifies Kr and the input retains its original SHA-256.
- **The same seven tests passed at `/rexafs/`**, with
  `SITE_URL=https://ameyanagi.github.io SITE_BASE=/rexafs`. This includes the real
  Worker calculation, asset loading and output downloads through the subpath.
- **Three parser and virtual-path tests passed.** They check column selection,
  malformed/nonfinite or unordered spectra, UTF-8, row limits, Unicode filenames
  and rejected traversal paths.
- The initial combined run passed the existing ten browser tests and the first
  six scattering tests (**16 total**). The subsequently added timeout test passed
  in the seven-test runs above.
- A later lifecycle review found that a stalled adapter import could outlast
  the five-minute timeout, and a restored page could retain disabled controls. After both
  fixes, **nine scattering tests passed at `/`**. The new tests verify recovery
  before the held module response is released, no abandoned Worker after late
  import completion, and successful retry. Persisted `pagehide`/`pageshow` events
  verify cancelled work and usable controls; this simulates lifecycle events,
  rather than establishing that Chromium cached an actual history navigation.

The [browser tests](../../../website/tests/browser/scattering.spec.ts) verify
finite spectra, the immutable input hash, the engine manifest and served WASM
hash, and the successful report. Downloads of `chi.dat`, `xmu.dat` and the binary
`pot.bin` match their exported byte counts and SHA-256 checksums. The calculation
record preserves root input text, auxiliary file bytes, input/output hashes,
engine identity, timing boundaries and structured errors.

Additional checks cover no outgoing uploads or off-origin requests during the
calculation, a responsive page during Worker progress, cancellation during
loading and execution, discarded stale outputs, isolated auxiliary files,
missing-include errors, a missing or altered WASM binary, and recovery with a
fresh calculation. A controlled browser clock verifies that the five-minute
budget includes engine loading; it does not wait five real minutes. Editing
input disables existing exports. Accessibility checks and a 390-pixel mobile
layout pass. The keyboard skip link appears on focus and remains outside the
ordinary viewport; screenshots are captured from the top of the page.

The [input tests](../../../website/tests/scattering-input.test.mjs) and
[workspace limits](../../../website/src/browser/scattering-input.ts) distinguish
interface safeguards from engine defaults: a 1 MiB root input, 64 auxiliary
files, 8 MiB total input, 128 MiB returned output, 10,000 output files and
100,000 plotted rows. Runs stop after five minutes or 2 MiB of received log text;
the displayed log retains its last 32,768 characters. These checks do not impose
a hard memory cap on WebAssembly, and large clusters remain outside this
qualification.

## Retained evidence and publication gate

The [staging script](../../../website/scripts/stage-refeff.mjs) checks the pinned
release archive, embedded source identity and selected file hashes before
replacing public assets. The distributed license files include the upstream
ReFEFF/FEFF10 notice, browser WASI shim licenses, a checksum-verified Cargo notice
inventory and the identified Rust/WASI runtime notices. Their
[maintenance record](../../../website/vendor/refeff-0.4.0/README.md) explains
source selection and the limits of those attribution inventories. Original
runtime, example and license file bytes are preserved.

After staging all notices, the production build and **22 content, input and
staging tests** passed, including nine staging checks. The two reference-generator
tests passed, and Astro checked 34 files with zero errors, warnings or hints.
The integration was committed as `20460174a5cd3fcd1ca0891ef92d7ee5db999e00` before
the subsequent website release promotion.

The final 0.2.5 website promotion passed all **21 browser tests at both `/` and
`/rexafs`**, including all nine scattering tests and both download-page tests.
Both builds also passed 22 content/input/staging tests and Astro's 35-file check
with zero errors, warnings or hints; both reference-generator tests passed.
The scientific comparison again covered all 4,010 values at the tolerances
above. Desktop and mobile screenshots were inspected for six-target downloads,
architecture requirements, plot labels and the unchanged input's Kr attribution.
Final local logs, outputs, screenshots and their checksum inventory are retained
in `/tmp/rexafs-0.2.5-final-website-qa/`, indexed by `verification.json`.

Native expected arrays and their provenance are retained in the repository
links above. The local raw native run, downloaded browser files, exported JSON,
comparison metrics, screenshots and command logs are retained separately under
`/tmp/rexafs-refeff-test-evidence/`, principally `native-znse/`, `browser-final/`
and `browser-subpath/`; `browser-renamed-input/` retains the final label check.
`browser-lifecycle/` retains the subsequent recovery regressions.
These temporary local files are not public release attachments. Browser/native
raw file hashes can differ while all numeric values satisfy the stated tolerance.

Before claiming a live deployment, merge the reviewed integration with green
checks, deploy the final website revision, and verify its published assets and
browser calculation. The immutable ReFEFF release identity above remains
separate from the rexafs package and desktop release version.
