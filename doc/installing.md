# Install rexafs

This guide targets 0.2.11. The [download page](https://rexafs.com/download/) lists
current packages and checksums.

## Desktop

Choose your platform on the [latest release](https://github.com/Ameyanagi/rexafs/releases/latest).

| Platform | Install |
|---|---|
| macOS ARM64 / x86-64 | Open the DMG, drag rexafs to Applications, then eject the DMG. Choose Apple Silicon or Intel. |
| Windows x86-64 / ARM64 preview | Run the matching setup executable, or extract its ZIP for a portable copy. |
| Linux x86-64 / ARM64 preview | Extract the matching archive and run `./rexafs` from its folder. See the runtime requirements below. |

The latest-release link follows new stable versions automatically. For the six
desktop targets in 0.2.11, see its
[qualification record](validation/2026-09-19-release-0.2.11/review.md). Windows ARM64
requires Windows 11 because its FEFF10 helper runs as an x64 process; rexafs and
ReFEFF themselves are native. See [native development](desktop-development.md)
for source builds.

### Linux portable archive

Download `rexafs-VERSION-x86_64-unknown-linux-gnu.tar.gz` for Intel/AMD or
`rexafs-VERSION-aarch64-unknown-linux-gnu.tar.gz` for ARM64 from the
[latest release](https://github.com/Ameyanagi/rexafs/releases/latest), where
`VERSION` is the release number. Extract it, open a terminal in the extracted
folder, and run:

```sh
./rexafs
```

Keep the folder's resources, examples and licenses beside the executable. This
is a portable application folder: it does not install a system package, register
a menu shortcut or require a Rust/Python runtime. To remove it, delete the
application folder after keeping your saved projects elsewhere.

Linux builds use Ubuntu 24.04 and require glibc 2.39 or newer, a graphical
session, a Vulkan-capable driver, GTK 3, fontconfig and xkbcommon with X11 support.
The `.tar.gz` does not bundle these system libraries. On Ubuntu 24.04, install
the runtime packages with:

```sh
sudo apt-get update
sudo apt-get install -y libgtk-3-0t64 libxkbcommon-x11-0 libvulkan1 \
  libfontconfig1 fonts-dejavu-core xdg-desktop-portal xdg-desktop-portal-gtk
```

Use the Vulkan driver for your graphics device. Run rexafs inside your graphical
login session so D-Bus and the file chooser portal are available. Other
distributions need compatible library versions; source-build dependencies are
listed in [Linux development](desktop-development.md#linux).
An x86-64 archive cannot run natively on an ARM64 system such as DGX Spark.

### Validation and checksums

ZIP archives remain available for portable installation and the desktop updater.
Checksums and installer evidence accompany the downloads. Native Windows
development-build interaction checks are recorded in the
[Windows review](validation/2026-09-09-windows-gui/README.md). Final-download
Windows interaction checks for the new controls remain outstanding. Linux release
checks use X11 with Mesa software rendering. The
[historical 0.2.4 qualification record](validation/2026-09-09-release-0.2.4/review.md) additionally
records local ARM64 checks with an NVIDIA GB10 hardware device and virtual X11
display. Physical monitors, native Wayland and clean-machine graphical setup
remain outside those checks. See the
[Linux review](validation/2026-09-09-linux-desktop/README.md) for the earlier
interaction results.

## Packages

For Python, create a project with the current stable
[uv](https://docs.astral.sh/uv/getting-started/installation/):

```sh
uv init --python 3.12 rexafs-analysis
cd rexafs-analysis
uv add rexafs==0.2.11
uv run python -c "import rexafs; print(rexafs.__version__)"
```

`uv add` records the dependency in `pyproject.toml`, updates `uv.lock`, and
prepares the project environment. `uv run` uses that environment without manual
activation. Keep the project files with your analysis scripts; see
[uv's project workflow](https://docs.astral.sh/uv/guides/projects/).

For TypeScript/JavaScript, use the current stable
[Bun](https://bun.sh/docs/installation) and run `bun add rexafs@0.2.11` in your
application directory. For Rust, use `cargo add rexafs@0.2.11` in a Cargo project.
Package registries supply the Python wheels, source distribution, npm/Wasm
package, and Rust crate; these are not duplicated in the desktop release assets.

## Offline installation

Desktop installers/archives can be copied to another computer of the same
platform. Keep portable archive contents together.

For Python, prepare uv and the Python interpreter on the offline computer first.
Use a connected computer with the same OS, architecture and Python version to
download wheels. This example uses Python 3.12; change it to the version on the
destination. uv does not provide a `pip download` subcommand, so use pip in a
temporary uv-managed environment for this preparation step:

```sh
uv run --no-project --python 3.12 --with pip python -m pip download --only-binary=:all: rexafs==0.2.11 --dest wheelhouse
```

On the offline computer, create the project:

```sh
uv init --offline --python 3.12 rexafs-analysis
cd rexafs-analysis
```

Copy `wheelhouse` into this project directory, then add the dependency and run
Python:

```sh
uv add --offline --no-index --find-links wheelhouse rexafs==0.2.11
uv run --offline python -c "import rexafs; print(rexafs.__version__)"
```

The `--offline` option prevents network access; the Python interpreter must be
installed already. Keep `pyproject.toml`, `uv.lock`, `.python-version` and the
wheelhouse with the project so its environment can be recreated. These wheels
target the machine/Python version selected during download; they are not a
complete cross-platform package archive. See [uv's project
workflow](https://docs.astral.sh/uv/guides/projects/).

For TypeScript/JavaScript, install Bun on the destination first. Download the
[0.2.11 package archive](https://registry.npmjs.org/rexafs/-/rexafs-0.2.11.tgz)
on the connected computer, copy it to your project on the offline computer, and run:

```sh
bun add ./rexafs-0.2.11.tgz
```

The rexafs 0.2.11 archive includes its WebAssembly binaries and has no runtime
package dependencies. Your application may have other dependencies that need to
be prepared separately. Bun supports [installing local
tarballs](https://bun.sh/docs/pm/cli/add). For Rust, use `cargo vendor` in your
consuming project and retain its generated Cargo configuration before building
offline.
