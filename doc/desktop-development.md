# Linux and Windows development

Use the Rust toolchain pinned by `rust-toolchain.toml`, Python 3.12+ for repository
tools, and Node 22+ for the optional JavaScript package.

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

The desktop supports X11 and Wayland and requires a Vulkan driver. Native file
dialogs require a running session D-Bus and an XDG desktop portal file chooser
backend. GNOME/KDE installations normally provide their own backend; the GTK
backend supports other desktops. Run the app inside your graphical login session.

The initial window fits the display's available area, including scaled laptop
screens. Numeric fields use DejaVu Sans Mono on Linux. Keep the extracted release
directory together so the executable can find its `resources` and licenses.

## Windows

Install Visual Studio Build Tools with **Desktop development with C++**, the
Windows SDK, Rust's MSVC toolchain, CMake, and Python 3.12+. In a developer shell:

```powershell
cargo test --locked -p rexafs
cargo check --locked -p rexafs --all-targets
cargo test --locked -p rexafs-gui --no-default-features --features refeff-runner
cargo run --locked --release -p rexafs-gui --no-default-features --features refeff-runner
```

FEFF10's current Windows prebuilt targets MinGW, so MSVC builds use ReFEFF.
Criterion benchmarks build on Windows; optional pprof flamegraphs require Unix.
Windows fields use Consolas, and shortcuts use Ctrl. See
[Windows installation](windows-installers.md) for the installer and portable ZIP.

## Repository checks

```bash
uv tool install pre-commit==4.5.1
pre-commit install --install-hooks
pre-commit run --all-files
pre-commit run --all-files --hook-stage pre-push
```

The configuration installs both commit and push hooks. Commit checks are read-only:
run `cargo fmt --all` to repair Rust formatting, then stage and commit again.
Push checks run the numerical core tests and strict Clippy when Rust sources or
dependency manifests change. Native GUI tests run in the release CI matrix,
where platform libraries and calculation backends are installed.

`python scripts/check-tooling.py` runs all release-tool test scripts. Their names
contain hyphens, so standard `unittest discover` does not collect them.
