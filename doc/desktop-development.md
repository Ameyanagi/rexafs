# Linux and Windows development

Run the commands below from the repository root. Use the Rust toolchain pinned
by [`rust-toolchain.toml`](../rust-toolchain.toml), Python 3.12+ for repository
tools, and Node 22+ for the optional JavaScript package. Release and website CI
currently use Node 24. Install [uv](https://docs.astral.sh/uv/getting-started/installation/)
to run the Python tooling commands.

## Linux

On Ubuntu 24.04, install the compiler and native development libraries:

```bash
sudo apt-get update
sudo apt-get install -y build-essential clang cmake pkg-config \
  libasound2-dev libfontconfig-dev libglib2.0-dev libgtk-3-dev libssl-dev \
  libvulkan-dev libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev \
  libzstd-dev libsqlite3-dev fonts-dejavu-core \
  xdg-desktop-portal xdg-desktop-portal-gtk
cargo test --locked -p rexafs
cargo test --locked -p rexafs-gui
cargo run --locked --release -p rexafs-gui
```

Use these commands on either x64 or ARM64 hardware with the corresponding
native Rust toolchain. The FEFF10 dependency supplies a prebuilt Linux ARM64
archive. The next-release CI adds `ubuntu-24.04-arm`; it must pass the packaged
engine and graphical checks before an ARM64 download is released. Published
0.2.4 has no Linux ARM64 desktop archive.

The desktop supports X11 and Wayland and requires a Vulkan driver. Native file
dialogs require a running session D-Bus and an XDG desktop portal file chooser
backend. GNOME/KDE installations normally provide their own backend; the GTK
backend supports other desktops. Run the app inside your graphical login session.

The initial window fits the display's available area, including scaled laptop
screens. Numeric fields use DejaVu Sans Mono on Linux. Keep the extracted release
directory together so the executable can find its `resources` and licenses.

For a repeatable graphical smoke test without a physical display:

```bash
sudo apt-get install -y xvfb xauth xdotool imagemagick mesa-vulkan-drivers
uv run --no-project --with Pillow python scripts/smoke-linux-gui.py --help
xvfb-run -a -s "-screen 0 1366x768x24" uv run --no-project --with Pillow \
  python scripts/smoke-linux-gui.py target/release/rexafs \
  crates/rexafs-gui/tests/fixtures/projects/rexafs-0.2.3-embedded.rxs
```

The test opens a temporary copy of an embedded project, switches processing
stages with Ctrl shortcuts, resizes the window, and quits. Screenshots and logs
are saved under `target/gui-smoke/`. This uses X11 and can use Mesa software
rendering; it does not qualify native Wayland or physical GPU performance.

## Windows

Install Visual Studio Build Tools with **Desktop development with C++**, the
Windows SDK, Rust's MSVC toolchain, CMake, and Python 3.12+. In a developer shell:

```powershell
python scripts/feff10_worker.py target/feff10-helper
$env:REXAFS_FEFF10_EXECUTABLE = "$PWD\target\feff10-helper\feff10-rs.exe"
cargo test --locked -p rexafs
cargo check --locked -p rexafs --all-targets
cargo test --locked -p rexafs-gui
cargo run --locked --release -p rexafs-gui
```

For an ARM64 source build, use Windows 11 ARM64, the Visual Studio ARM64 C++
tools and the native Rust toolchain. Before the commands above, select it with:

```powershell
rustup toolchain install 1.98.1-aarch64-pc-windows-msvc
$env:RUSTUP_TOOLCHAIN = "1.98.1-aarch64-pc-windows-msvc"
$env:REXAFS_EXPECTED_TARGET = "aarch64-pc-windows-msvc"
```

The expected target is checked by desktop packaging. A native ARM64 rexafs
executable and ReFEFF backend still use the existing x64 FEFF10 helper.
[Windows 11 provides x64 application emulation](https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-x86-emulation);
Windows 10 ARM64 does not. The next-release matrix tests this mixed setup on
`windows-11-arm`, including installed FEFF10 calculations. Adding the job is
not evidence that it passed; native ARM64 qualification remains required.

FEFF10's Windows prebuilt targets MinGW, which the MSVC desktop cannot link.
Windows builds therefore keep the `feff10-runner` feature but run FEFF10
through the upstream `feff10-rs.exe` command-line helper as a separate
process, one fresh process per FEFF stage. `scripts/feff10_worker.py`
downloads that executable and its MinGW runtime DLLs from the pinned feff10-rs
release and verifies their SHA-256 values. The runner looks for the helper in
`REXAFS_FEFF10_EXECUTABLE`, then in `resources/feff10` beside `rexafs.exe`
(where release packaging installs it), then on `PATH`. Without it, FEFF10
calculations report a missing `feff10-rs` executable while ReFEFF keeps working.
Criterion benchmarks build on Windows; optional pprof flamegraphs require Unix.
Windows fields use Consolas, and shortcuts use Ctrl. See
[Windows installation](windows-installers.md) for the installer and portable ZIP.

## Repository checks

See [CI selection and release coverage](ci.md) for changed-file selection,
aggregate checks and the measured build bottlenecks.

```bash
uv tool install --python 3.12 pre-commit==4.5.1
pre-commit install --install-hooks
pre-commit run --all-files
pre-commit run --all-files --hook-stage pre-push
```

The configuration installs both commit and push hooks. Commit checks are read-only:
run `cargo fmt --all` to repair Rust formatting, then stage and commit again.
Push checks run the numerical core tests and strict Clippy when Rust sources or
dependency manifests change. Native GUI tests run in the release CI matrix,
where platform libraries and calculation backends are installed.

`uv run --no-project --python 3.12 python scripts/check-tooling.py` runs every
`scripts/test-*.py` suite with the same interpreter. Their names contain hyphens,
so standard `unittest discover` does not collect them. The Node package and
platform installer smoke tests have separate runners.

The checks have different scopes; a passing core suite does not verify the
installed bindings, website, or native desktop:

| Area | Source of the check commands | What it verifies |
|---|---|---|
| Core Rust | [Rust workflow](../.github/workflows/rust.yml) | Default and optional numerical backends, plotting, Windows compilation, formatting, strict Clippy, and benchmark regression gates |
| Release packages | [Release workflow](../.github/workflows/release-build.yml) and [release runbook](releasing.md) | Extracted Rust package, Python wheels and source archive, npm tarball, desktop archives, and platform smoke tests |
| Editor help | [Binding checks](../js-rexafs/test/README.md) | Completion, signatures and hover text from the installed Python and TypeScript packages |
| Public documentation | [Website maintenance](../website/README.md) and [website workflow](../.github/workflows/website.yml) | Generated API reference, citations, links, browser behavior, and deployment |

Benchmark regressions are informational on pull requests and blocking on pushes
to `main`. Native rendering and platform installation require their own checks;
archive self-checks only exercise the calculation and packaging paths.
