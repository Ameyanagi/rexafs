# Install rexafs

## Desktop

Choose your platform on the [latest release](https://github.com/Ameyanagi/rexafs/releases/latest).

| Platform | Install |
|---|---|
| macOS ARM64 / x86-64 | Open the DMG, drag rexafs to Applications, then eject the DMG. Choose Apple Silicon or Intel. |
| Windows x86-64 preview | Run the setup executable, or extract the ZIP for a portable copy. |
| Linux x86-64 preview | Extract the archive and run `./rexafs` from the extracted folder. See the runtime requirements below. |

The latest-release link follows new stable versions automatically. Linux and
Windows ARM64 downloads are not currently published. Linux ARM64 can be built
from source; Windows ARM64 has not been qualified.

### Linux portable archive

Download the `rexafs-VERSION-x86_64-unknown-linux-gnu.tar.gz` asset from the
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
Windows/Linux interactive qualification and native Intel hardware checks remain
outstanding.

## Packages

```sh
python -m pip install rexafs
npm install rexafs
cargo add rexafs
```

Use the command for your project. Package registries supply the Python wheels,
source distribution, npm/Wasm package, and Rust crate; these are not duplicated
in the desktop release asset list.

## Offline installation

Desktop installers/archives can be copied to another computer of the same
platform. Keep portable archive contents together.

For Python, download on a connected computer with the same OS, architecture,
and Python version as the offline computer:

```sh
python -m pip download rexafs==0.2.3 --dest wheelhouse
```

Copy `wheelhouse` to the offline computer, then run:

```sh
python -m pip install --no-index --find-links wheelhouse rexafs==0.2.3
```

For npm, `npm pack rexafs@0.2.3` downloads the package for a later
`npm install ./rexafs-0.2.3.tgz`. For Rust, prepare the consuming project's
dependencies with `cargo vendor`; retain its generated configuration alongside
the project's existing Cargo settings before building with `--offline --locked`.
