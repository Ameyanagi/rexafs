# Install rexafs

## Desktop

Choose your platform on the [latest release](https://github.com/Ameyanagi/rexafs/releases/latest).

| Platform | Install |
|---|---|
| macOS | Open the DMG, drag rexafs to Applications, then eject the DMG. Choose Apple Silicon or Intel. |
| Windows preview | Run the setup executable, or extract the ZIP for a portable copy. |
| Linux preview | Extract the archive and run `rexafs` from the extracted folder. A graphical session, Vulkan-capable driver, GTK 3, fontconfig, and xkbcommon are required. |

ZIP archives remain available for portable installation and the desktop updater.
Checksums and installer evidence accompany the downloads. Native Windows
development-build interaction checks are recorded in the
[Windows review](validation/2026-09-09-windows-gui/README.md). Final-download
Windows interaction checks for the new controls remain outstanding. Linux GUI
checks use X11 with Mesa software rendering; physical GPUs, native Wayland,
clean-machine graphical installation and native Intel hardware remain unqualified.
See the [Linux review](validation/2026-09-09-linux-desktop/README.md).

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
python -m pip download rexafs==0.2.4 --dest wheelhouse
```

Copy `wheelhouse` to the offline computer, then run:

```sh
python -m pip install --no-index --find-links wheelhouse rexafs==0.2.4
```

For npm, `npm pack rexafs@0.2.4` downloads the package for a later
`npm install ./rexafs-0.2.4.tgz`. For Rust, prepare the consuming project's
dependencies with `cargo vendor`; retain its generated configuration alongside
the project's existing Cargo settings before building with `--offline --locked`.
