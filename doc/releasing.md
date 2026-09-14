# Releasing rexafs

## Preparing 0.2.6

The [0.2.6 notes](release-notes-0.2.6.md) and
[qualification record](validation/2026-09-14-release-0.2.6/review.md) track the
shared measurement reader, plotted import workflow and coordinated package
release. Build the immutable `v0.2.6` tag through GitHub before publishing any
channel. Website Stable metadata remains on 0.2.5 until the new artifacts and
registry uploads have been verified.

## Historical preparation of 0.2.5

The [0.2.5 notes](release-notes-0.2.5.md) and
[qualification record](validation/2026-09-13-release-0.2.5/review.md) track the
browser preview, simpler processing APIs and six desktop targets. Publication
requires a successful manual build of the immutable tag after PR #58 is merged.
The website's Stable release stays on 0.2.4 until the new packages are published.

`scripts/check-release-version.py` checks Cargo's workspace, local lockfile
entries, npm's manifest and lockfile, and Python's inherited version. Passing
`v0.2.5` also checks the tag; release CI does this before packaging. Advance the
website's release metadata separately after verifying published artifacts, then
regenerate Stable references from that tag and check the deployed install links.

## Historical preparation of 0.2.4

The 0.2.4 patch release improved desktop sizing, empty-workspace actions, text
editing and bounded folder scanning. It added Windows core validation and an
X11 smoke check of the packaged Linux application. Version 0.2.3's existing tag
is preserved. The [release notes](release-notes-0.2.4.md) and
[qualification record](validation/2026-09-09-release-0.2.4/review.md) track this
release's source, checks and publication. Numerical defaults were unchanged;
the release also included ureq's 3.4.1 HTTP fixes. The [compatibility fixes](fft-grid-compatibility.md)
correct legacy derivatives and add an explicit FFT grid choice. The [0.2.3 record](validation/2026-09-10-release-0.2.3/review.md)
retains that release's completed build, registry, signing and publication history.

## Published releases

Use the [latest stable release](https://github.com/Ameyanagi/rexafs/releases/latest)
for current desktop downloads and publication status. The
[installation guide](installing.md) links the package channels. Each release's
notes and qualification record identify its exact source, build, signatures and
checks; preparation records do not establish publication.

The [0.2.1 notes](release-notes-0.2.1.md) and
[qualification report](validation/2026-09-09-release-0.2.1/review.md) retain the
completed publication history for that version. Windows and Linux remain desktop
previews with the limits stated in each release's qualification record.

Version 0.2.2 was withheld after its Linux build failed; its tag remains immutable.
The desktop quality work in PRs #48 and #49 is outside the 0.2.3 source tag.

The [rebranding plan](rebranding-plan.md) defines scope; [dependency notes](dependencies.md)
record the Rust 1.98.1 toolchain and compatibility constraints.

## Historical 0.1.0 launch record

The following records the initial launch as checked on 2026-09-06. Later releases
completed trusted publishing and added Windows/Linux previews; use the linked
release reports for current status.

- The public repository is now [`Ameyanagi/rexafs`](https://github.com/Ameyanagi/rexafs),
  and this checkout's `origin` and current package metadata use that name.
- [Release build 34012430217](https://github.com/Ameyanagi/rexafs/actions/runs/34012430217)
  passed for PR #16 at `a88599ee382fb7a4ed89dce48b094b160bc5cc35`. All build jobs
  succeeded, including 20 Python wheel targets and four desktop targets; all 28
  uploaded artifacts remain available. This was a pull-request run, so it cannot
  be promoted by `publish.yml`.
- [Main CI 34013687691](https://github.com/Ameyanagi/rexafs/actions/runs/34013687691)
  passed at merge commit `df7a2d698ba1e4d39848b6a452c05bc180423937`.
- PR #17 was merged at `ee365067ba97a762888caf65593af213dca5b7e4`, and `v0.1.0`
  points to that commit. [Tagged build 34025866097](https://github.com/Ameyanagi/rexafs/actions/runs/34025866097)
  passed all 29 jobs; main Rust CI passed all four jobs.
- crates.io and npm now contain `rexafs 0.1.0`. PyPI contains all 20 wheels and
  the source distribution. [Source repair 34028288522](https://github.com/Ameyanagi/rexafs/actions/runs/34028288522)
  restored three omitted root license files, passed installation/API tests, and
  uploaded the sdist without changing existing source files, wheels or the tag.
  All published package hashes were checked against their GitHub artifacts.
- crates.io and npm trusted publishers were registered and verified in their
  settings: `Ameyanagi/rexafs`, `publish.yml`, environment `release` (npm permits
  direct `npm publish`). The first uploads used the bootstrap tokens; an OIDC
  upload for these two registries has not yet been exercised.
- [GitHub release v0.1.0](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.0)
  became public on 2026-09-06 with 30 verified assets. Both macOS archives were
  signed and notarized by [run 34032364680](https://github.com/Ameyanagi/rexafs/actions/runs/34032364680),
  using Developer ID team `XXN44W8X56`. The signed Apple Silicon app passed local
  plot rendering and project save/reopen checks; the Intel app passed launch,
  plot rendering and project reopen under Rosetta. Native Intel graphical
  hardware was not tested locally. Windows/Linux desktop archives remain in CI
  pending graphical qualification, and are excluded from the public release.
- The GitHub `release` environment is configured for tags matching `v*`.
  Its `CRATES_IO_TOKEN` and `NPM_TOKEN` secret names were verified through GitHub;
  their values were not read. The maintainer confirmed the PyPI pending publisher
  for `rexafs`, repository `Ameyanagi/rexafs`, workflow `publish.yml`, environment
  `release`. Registry authentication will be exercised by the first tagged upload.

The [0.1.0 release notes](release-notes-0.1.0.md) preserve the original launch
scope. Windows/Linux preview distribution began with 0.1.3, and native
interactive qualification remains outstanding. Website/domain deployment is
separate from the package release process.

## Local checks

Run these commands from the repository root, with the pinned Rust toolchain and
Python 3.12+ for repository tools. Local checks support review; the
[release workflow](../.github/workflows/release-build.yml) defines the complete
platform and package qualification matrix.

```bash
python scripts/check-compatibility-fixtures.py
cargo fmt --all -- --check
cargo deny --locked check licenses
cargo test --locked --manifest-path vendor/sum_tree/Cargo.toml
cargo test --locked -p rexafs
cargo test --locked -p rexafs --features ndarray-compat
cargo test --locked -p rexafs --features trust-region
cargo clippy --locked -p rexafs --all-targets -- -D warnings
cargo check --locked -p rexafs --features ndarray-compat
cargo check --locked -p rexafs --features plotting,refeff-runner,feff10-runner,amcsd,materials-project,cod
cargo doc --locked -p rexafs --no-deps
cargo package --locked -p rexafs
```

Inspect the `.crate` contents, licenses and required compressed structure assets.
Cargo verifies the extracted package without relying on the workspace's GPUI patch.
Do not use `--no-verify` as the final publishing gate. A dirty checkout may be
packaged locally with `--allow-dirty` for review; releases require a reviewed commit.

Python:

```bash
uv venv --python 3.12 /tmp/rexafs-wheel-check
uvx maturin build --release --locked --manifest-path py-rexafs/Cargo.toml \
  --interpreter /tmp/rexafs-wheel-check/bin/python --out /tmp/rexafs-release-check
uvx maturin sdist --manifest-path py-rexafs/Cargo.toml --out /tmp/rexafs-release-check
uv pip install --python /tmp/rexafs-wheel-check/bin/python /tmp/rexafs-release-check/rexafs-*.whl
/tmp/rexafs-wheel-check/bin/python py-rexafs/tests/test_api.py
npm --prefix js-rexafs ci
REXAFS_PYTHON=/tmp/rexafs-wheel-check/bin/python npm --prefix js-rexafs run test:python-editor
```

These shell examples use POSIX temporary paths. On Windows, use an empty
directory under `$env:TEMP` and the environment's `Scripts/python.exe`. Choose
fresh output directories so a wildcard cannot select wheels from an earlier build.
Use another fresh environment to rebuild/install the sdist, run
`scripts/check-python-sdist.py` on it, and run the same runtime tests. Test each
supported CPython minor (3.10–3.14), OS and architecture. Linux
wheels must meet the declared manylinux policy; a local Linux wheel is insufficient.

For the unreleased ABI3 source profile, build once per platform and install those
same wheel bytes on all five interpreters. Run `scripts/check-python-wheels.py`
against the wheel directory before and after installation (`--installed`). Test
both the latest compatible NumPy and these minimum binary releases:

| CPython | Minimum NumPy wheel |
|---|---|
| 3.10 | 1.23.0 |
| 3.11 | 1.23.4 |
| 3.12 | 1.26.0 |
| 3.13 | 2.1.0 |
| 3.14 | 2.3.3 |

Use `--numpy-version VERSION` to verify each installed minimum. NumPy has its
own binary compatibility requirements; CPython's stable ABI does not replace
these runtime checks. See [NumPy's downstream guidance](https://numpy.org/doc/stable/dev/depending_on_numpy.html).

JavaScript:

```bash
mkdir -p /tmp/rexafs-release-check
npm --prefix js-rexafs ci
cargo install wasm-pack --locked --version 0.15.0
npm --prefix js-rexafs run build
npm --prefix js-rexafs test
npm pack --prefix js-rexafs ./js-rexafs --pack-destination /tmp/rexafs-release-check
node scripts/test-npm-package.mjs /tmp/rexafs-release-check/rexafs-*.tgz
```

Install the tarball in a fresh consumer project, type-check its public imports,
and exercise both the Node and browser entry points with a real spectrum. Include
the `.wasm` files and license notices; no compiler is required on the consumer side.

Desktop on macOS or Linux (run on the target platform):

```bash
cargo test --locked --release -p rexafs-gui --no-default-features --features refeff-runner,feff10-runner
cargo build --locked --release -p rexafs-gui --no-default-features --features refeff-runner,feff10-runner
python scripts/test-release-archive.py
python scripts/package-desktop.py
```

On Windows, use the same `refeff-runner,feff10-runner` features. The MSVC
desktop cannot link the upstream MinGW FEFF10 archive, so packaging downloads
the verified `feff10-rs.exe` helper and its runtime DLLs into `resources/feff10`,
and the packaged `--self-check-feff` runs FEFF10 through that helper. For the
source-tree tests, stage the helper first with `python scripts/feff10_worker.py
target/feff10-helper` and point `REXAFS_FEFF10_EXECUTABLE` at it; see the
[Windows development instructions](desktop-development.md#windows).

The 0.2.5 matrix adds native Linux and Windows ARM64 builds. Linux uses
the upstream ARM64 FEFF10 archive. Windows ARM64 builds rexafs and ReFEFF natively
but runs the x64 FEFF10 helper through Windows 11 emulation; this must not be
reported as native ARM64 FEFF10. `build.json` records each engine's execution
mode. The packaged and installed self-checks must pass on the matching native
runner before releasing either new target. Published 0.2.4 has neither package.

Use Python 3.12+ for the release scripts. `package-macos.sh` remains a macOS build
convenience wrapper. Archives go to `target/distributions/` with version and Rust
host triple in the filename: macOS `.app` ZIP, Linux `.tar.gz`, Windows ZIP.
All archives contain both ReFEFF and FEFF10, an example, license files, a dependency inventory and build
metadata, with an adjacent SHA-256 checksum; Windows carries FEFF10 as the helper process. The script extracts the archive into
a fresh directory and runs its executable's `--version` and `--self-check`.
The latter processes the packaged example without relying on the source checkout.
The additional `--self-check-feff` runs each compiled engine on fcc Cu and checks
first-shell geometry and amplitudes, including the FEFF10 worker process route.
Neither check tests GPU rendering or replaces an interactive launch check.
Windows ZIP packaging clamps upstream file dates to ZIP's supported range
(1980–2107), preserving file contents and leaving source timestamps untouched.
It stages the matching, signed Microsoft C++ runtime before executing the copied
application and records DLL provenance in `build.json`. The installer revalidates
and preserves those same files, so the ZIP and installer use the same runtime.

The macOS package rejects Homebrew/local dynamic libraries; Linux rejects unresolved
linked libraries. Inspect `linked-libraries.txt` and qualify a clean installation.
The Linux runner installs GPUI's X11/Wayland build dependencies; the release needs
a graphical session, GTK 3, fontconfig, xkbcommon and a Vulkan-capable driver. Windows
uses its native MSVC/Windows SDK toolchain. Test the actual minimum OS before
advertising compatibility beyond the runner image.

The build matrix produces unsigned archives. `sign-macos.yml` signs the two macOS
archives from an existing qualified build, without rebuilding their executables.
It checks source-run identity, original checksums and bundle metadata before using
the signing identity. It applies Developer ID signing with Hardened Runtime and a
timestamp, requires an accepted Apple notarization submission, staples the app,
then builds the final ZIP and checksum. A fresh extraction must pass codesign,
stapler, Gatekeeper, `--version` and `--self-check` checks. Windows remains unsigned.
Qualify the signed download on a clean machine. Preserve the
[dependency notices](distribution-notices.md) for each distribution.

The `macos-signing` environment permits only `main`. Its secrets are
`MACOS_CERTIFICATE_P12_BASE64`, `MACOS_CERTIFICATE_PASSWORD`, `APPLE_ID` and
`APPLE_APP_SPECIFIC_PASSWORD`; its variables are `APPLE_TEAM_ID` and
`MACOS_SIGNING_IDENTITY`. The signing workflow uses a temporary keychain and
deletes it in a `finally` block. Only signed archives and their checksums are
uploaded. Credentials are never release artifacts. Setup follows
[Apple's notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
and [GitHub's certificate guidance](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications).

```bash
gh workflow run sign-macos.yml --ref main -f release_tag=v0.1.0 -f build_run_id=34025866097
```

The signing run records its own run ID, signing-tools commit, original build
commit/run ID, unsigned archive hash and Apple submission ID inside `build.json`.
When preparing a GitHub draft, replace both original macOS ZIPs and their checksum
files with the signing run's outputs and regenerate `SHA256SUMS`. Record both build
and signing runs in the release notes. Do not promote the unsigned matrix ZIPs as
the signed downloads.

### Completing the rejected 0.1.0 Python source upload

`repair-pypi-sdist.yml` runs only from `main`, validates the successful tagged source
build, and restores the missing `License-File` entries from that exact Git commit.
It checks that existing archive members are byte-identical, checks license paths,
installs from the repaired sdist and runs the tagged Python API tests. Its publish
job uploads only the previously missing sdist using a separate PyPI trusted
publisher: `Ameyanagi/rexafs`, workflow `repair-pypi-sdist.yml`, environment
`pypi-repair`. That environment permits only `main`.

```bash
gh workflow run repair-pypi-sdist.yml --ref main -f release_tag=v0.1.0 -f build_run_id=34025866097
```

Keep the repaired source artifact/checksum from this GitHub run for the draft
release, and record its run ID alongside the original build. Future builds include
all three root licenses through `tool.maturin.include` and explicitly verify the
sdist's declared license paths before upload. This repair is limited to missing
license files in a rejected asset; source code changes require a new version.

## GitHub is the release build authority

Local artifacts are verification outputs, not public release uploads.
`release-build.yml` runs on pull requests and supports manual/reusable execution.
It builds and tests:

- the non-GPL dependency license policy across all features and platform branches,
  plus the Apache sum_tree patch's upstream tests;
- the Rust source crate on Ubuntu;
- four `cp310-abi3` wheels on Ubuntu x64, Apple Silicon macOS, Intel macOS and
  Windows x64, with all 20 CPython 3.10–3.14 runtime combinations testing the
  downloaded wheel against minimum and latest compatible NumPy;
- the Python sdist, including an installation rebuilt from that source archive;
- the npm tarball, Node/browser numerical tests and an installed TypeScript consumer;
- desktop archives on macOS 15 ARM64/Intel, Ubuntu 24.04 x64/ARM64 and Windows
  x64/ARM64, using the native `ubuntu-24.04-arm` and `windows-11-arm` runners for
  the new targets. These labels are listed in the official
  [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

Desktop jobs select a host-specific Rust toolchain and require its target to
match the matrix. Packaging checks the executable's machine type, then tests
the extracted archive. Linux GUI evidence uses a separate archive per target so
its image/log filenames cannot collide in the flat release manifest. Python's
wheel matrix remains separate; adding a desktop target does not add a wheel.

The four-wheel ABI3 profile is **unreleased**; 0.2.5 retains its 20 published
per-interpreter wheels. PyO3's `abi3-py310` feature selects the CPython stable
binary interface with a 3.10 minimum. This qualification covers GIL-enabled
interpreters only; see [PyO3's ABI documentation](https://pyo3.rs/v0.29.2/building-and-distribution.html#py_limited_apiabi3abi3t).
The workflow verifies wheel names, metadata, platform baselines and installed
package bytes, preserves the Linux 3.12 editor check, and checks that the sdist
retains the stable-ABI feature before rebuilding and testing it. Publication
selects the ABI profile from the immutable source tag: new ABI3 releases require
exactly four wheels, while historical tags retain their original manifest
inventory. A future versioned release must qualify this profile before upload;
do not replace or relabel 0.2.5 assets.

The final manifest job requires **every** build and runtime job to succeed. `SHA256SUMS` uses
flat asset names so it also works after downloading all GitHub Release assets into
one directory. A matrix entry is a qualification target, not evidence of support;
record the actual successful run and perform GUI launch/import/process/project
reopen/ReFEFF checks on each advertised target.

After the final repository rename and reviewed version tag, dispatch the build:

```bash
gh workflow run release-build.yml --ref v0.1.0
gh run list --workflow release-build.yml
# After the selected build succeeds:
gh workflow run publish.yml --ref v0.1.0 -f channel=github-draft -f build_run_id=RUN_ID
```

`publish.yml` requires the matching version tag and a successful, manually dispatched
`release-build.yml` run for that exact commit. It downloads that run's artifacts and
verifies every checksum. The Rust publishing step additionally reproduces the
`.crate` in GitHub and compares its bytes before using Cargo's registry uploader.
Python and npm upload the downloaded distributions directly. Local artifact paths
are never accepted as publication inputs.

Registry channels download only their required artifacts and the original build
manifest. Rust requires the source crate, npm its tarball, and PyPI every wheel
listed in that manifest plus the source archive. Missing, additional, duplicate,
wrong-version, or modified packages fail verification. GitHub draft creation still
requires the complete artifact set. A desktop artifact transfer failure therefore
cannot block publication of already-qualified registry packages.

GitHub's public download list is desktop-only. `release_downloads.py` requires
all six platform archives, both Windows installers and their qualification
records for versions after 0.2.4. Historical versions through 0.2.4 retain their
four-target requirement. Checksums and available Mac installer evidence are
staged after verifying the original build hashes. Registry artifacts stay in
the successful build and their registries. The staged `SHA256SUMS` covers only
the files being uploaded; retain the original complete build manifest separately.
After signing, replace the Mac archives, add their installers/evidence, and
regenerate the public desktop manifest. Never mix unsigned and signed checksums.

Lead release notes with platform download links; put package commands, detailed
changes, and qualification evidence in collapsed sections. Link to
[installation instructions](installing.md) for offline setup.

The workflow normally runs on the release tag. To resume with corrected
publication tooling, review and merge the tooling change, create a separate
`vX.Y.Z-publish-tools.N` tag for that tooling commit, and supply the original
`release_tag` explicitly. The release environment continues to require a tag;
its permissions and trusted publishers are unchanged. Source validation resolves
the original tag and requires its matching successful manual build. Rust is
checked out at that resolved source commit before Cargo reproduces and compares
the crate. npm's `latest`/`next` choice also uses the source release tag.

```bash
gh workflow run publish.yml --ref v0.2.0-publish-tools.1 \
  -f release_tag=v0.2.0 -f channel=npm -f build_run_id=34204231697
```

Record both the publication-tooling ref and source build in the qualification
report. Keep the source release tag immutable; a tooling tag does not create a
new application release or replace its artifacts.

The `release` GitHub environment is configured for the final repository. Channels:

- `crates-io`: select `registry_auth=token` to use `CRATES_IO_TOKEN` for the first
  upload. After configuring the crate's trusted publisher, use the default
  `registry_auth=trusted-publishing` to obtain a temporary token through
  `rust-lang/crates-io-auth-action`. Package verification runs before authentication.
- `pypi`: pending/existing trusted publisher for `publish.yml`, environment `release`.
  This channel always uses trusted publishing and needs no PyPI API token.
- `npm`: select `registry_auth=token` to use `NPM_TOKEN` for the first upload.
  After configuring the package's trusted publisher, use the default
  `registry_auth=trusted-publishing`; it does not pass the bootstrap token to npm.
- `github-draft`: creates a draft with GitHub-built assets and checksums. Review
  platform qualification, notices, signing status and notes before making it public.

All registry publisher forms use these exact values:

| Field | Value |
|---|---|
| Package/project | `rexafs` |
| GitHub owner | `Ameyanagi` |
| Repository | `rexafs` |
| Workflow filename | `publish.yml` |
| Environment | `release` |

For the first coordinated upload, replace `RUN_ID` with the successful manual
release build of the `v0.1.0` commit:

```bash
gh workflow run publish.yml --ref v0.1.0 -f channel=crates-io -f registry_auth=token -f build_run_id=RUN_ID
gh workflow run publish.yml --ref v0.1.0 -f channel=npm -f registry_auth=token -f build_run_id=RUN_ID
gh workflow run publish.yml --ref v0.1.0 -f channel=pypi -f registry_auth=trusted-publishing -f build_run_id=RUN_ID
```

PyPI's pending publisher creates the project on first upload. crates.io and npm
require an existing package before their trusted publisher can be configured.
After those first uploads, add the publishers using the table above. For npm,
allow direct `npm publish`, which is the operation used by this workflow. Remove
the bootstrap secrets and revoke their registry tokens after switching to and
verifying trusted publishing. See the [PyPI setup guide](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/),
[crates.io guidance](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/)
and [npm trusted publishing guide](https://docs.npmjs.com/trusted-publishers/).

Repeat publication with the **same build_run_id and tag** for missing channels;
no rebuild is necessary after a registry-only failure. Keep the run's artifacts
until every channel completes. Never overwrite a published package version. A code
change requires a new coordinated version/tag and a new successful build. Release
candidates use Cargo/npm `0.1.0-rc.1`, Python `0.1.0rc1`, npm `next` and GitHub's
prerelease marker. A resumed draft upload may report already-present assets;
compare their hashes before replacing anything.

## Historical repository rename and launch checklist

This checklist records the initial launch sequence. The repository rename,
registry publication and GitHub Pages deployment have since been completed.
For documentation deployment, use the current [website maintenance guide](../website/README.md).

1. Finish local rebranding and artifact qualification.
2. The existing GitHub repository has been renamed from `ameyanagi/xraytsubaki`
   to `Ameyanagi/rexafs`, preserving its history and issue/PR context.
3. `origin` and current workspace, Python/npm and documentation URLs now use
   `https://github.com/Ameyanagi/rexafs`. Update other existing clones as needed.
   Retain old URLs only where they explain migration history.
4. Configure/reconcile registry trusted-publisher repository/workflow/environment
   bindings with the final name. Verify Actions access and tag protection.
5. Review version, changelog and package contents; push the release commit and tag.
   Run the GitHub multi-platform build, qualify its downloads, then publish each
   channel using that successful build run ID.
6. Deploy documentation to `rexafs.com` on the selected host, verify DNS/HTTPS and
   download links, then announce only the channels/targets that passed installation.

Public registry lookups on 2026-09-06 found no rexafs package; they do not reserve
names. Recheck before the first publish. Registry guidance is linked from the
[plan](rebranding-plan.md); authentication setup and final remote actions are still
maintainer release steps.

<a id="validation-record"></a>

## Historical validation record

Verified locally on Apple Silicon macOS with Rust 1.98.1, 2026-09-06:

| Check | Result |
|---|---|
| Core default test suite | 213 passed, 3 ignored |
| Core ndarray compatibility suite, including doctests | 250 passed, 3 ignored |
| Core trust-region suite | 213 passed, 3 ignored |
| Direct spline solve | Independent SciPy knot/coefficient/evaluation/Jacobian references passed; Ru/Cu/Ni χ differs from the previous pipeline by <4e-14 |
| Dependency licenses | cargo-deny 0.20.2 passed across all features/platform branches; standalone sum_tree license check passed |
| Apache sum_tree patch | 10 upstream tests passed |
| Core clippy, all targets, `-D warnings` | Passed |
| Rustdoc, `RUSTDOCFLAGS="-D warnings"` | Passed |
| Optional plotting/ReFEFF/FEFF10/database integrations | Compile checks passed |
| Extracted crates.io package | Cargo verification passed; source, licenses and compressed structure assets present |
| CPython 3.12 wheel | Fresh installation; 5 API tests passed |
| CPython 3.14 sdist rebuild | Fresh installation with NumPy 2.5.2; 5 API tests passed |
| npm/Wasm | 3 tests passed, Chromium fetch/processing passed, tarball installation and Node-only TypeScript compilation passed |
| Desktop optimized tests, ReFEFF build | 124 passed, 2 ignored; includes project fixtures, captioned exports, credential permissions and stale FEFF jobs |
| macOS app archive | Extracted `--version` / `--self-check` passed; example E0=8977.493 eV |
| macOS interactive app | Bundled Cu example rendered; custom 4 × 3 inch / 300 DPI PNG saved at 1200 × 900; project reopened with saved plot settings; opaque cluster correctly occluded the central absorber; .rxs controls saved both modes and a portable embedded copy restored the selected spectrum/overrides without adjacent inputs |
| Publication report | Five vector figures rendered in Chromium; numbered figure/table captions and resolved processing values verified visually |
| Project compatibility | Five .rxs fixtures, three raw inputs and a frozen defaults snapshot checksummed; linked relocation/Save As, lossless embedded recovery without originals, repeat saves, unchanged processing, metadata, backup/failure and exact-float tests passed |
| Release automation | actionlint passed; checksum roundtrip and tamper rejection passed; coordinated version check passed |
| Documentation | Relative-link audit passed; formatting and diff whitespace checks passed |

Upstream future-incompatibility notices remain for binrw 0.12.1, block 0.1.6 and
proc-macro-error2 2.0.1. The GUI also has existing unused-code warnings; these did
not fail its build/tests. Ignored scientific/performance tests are not counted as
passing. The ndarray suite exposed an overlength FFT input panic, now fixed to
match the default implementation's truncation.

Spline replacement exposed a joint-fit stopping issue near floating-point
resolution. The FEFF DogLeg parameter tolerance now matches its forward-difference
Jacobian step, `sqrt(f64::EPSILON)`; cost and gradient tolerances are unchanged.
The joint-fit regression still requires convergence, known parameter recovery and
finite positive uncertainties. AUTOBK's direct solver is unchanged.

The table above records **local** results. The subsequent GitHub pull-request
matrix passed, as recorded in the current launch status, and the repository rename
is complete. Automated archive self-checks do not qualify interactive Intel macOS,
Linux or Windows launches. Registries and the signed macOS desktop release are
now published, as recorded above. Domain deployment was pending when this record
was written; the current public manual is hosted at [rexafs.com](https://rexafs.com/). Future releases
require a successful manual build of their final tag and qualification of the
actual downloads, including each platform's notices.
