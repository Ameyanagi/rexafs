---
title: "Install rexafs"
description: "Install the desktop or stable Python, TypeScript and Rust packages."
audience: user
---

The desktop and prebuilt Python/npm packages do not require a Rust compiler.

## Desktop

Use the [download page](/download/) to select your operating system and processor.

| Platform | Install | Support |
|---|---|---|
| macOS Apple Silicon | Open the ARM64 DMG and drag rexafs into Applications | Signed and notarized |
| macOS Intel | Open the Intel DMG and drag rexafs into Applications | Signed and notarized |
| Windows x64 | Run the setup EXE; launch rexafs from the Start menu | Preview; Windows 10 2004 or newer / Windows 11 |
| Windows ARM64 | Run the ARM64 setup EXE; launch rexafs from the Start menu | Preview; Windows 11 |
| Linux x64 | Extract the archive; run `./rexafs` inside its folder | Preview; Ubuntu 24.04 runtime baseline |
| Linux ARM64 | Extract the ARM64 archive; run `./rexafs` inside its folder | Preview; Ubuntu 24.04 runtime baseline |

Keep portable folders together, including their resources, examples and licenses.
Save your project and close the application before replacing it.

The Windows installer uses your user directory and does not require administrator
rights. Uninstall through **Settings → Apps**; your `.rxs` projects remain separate.
The rexafs Windows executable and installer are not publisher-signed in this release.

### ARM64 availability

**ARM64 downloads are available for macOS, Windows and Linux in 0.2.6.**
Select your architecture on the [download page](/download/). Windows and Linux
remain desktop previews.

Linux ARM64 runs rexafs, ReFEFF and FEFF10 natively. Windows ARM64 runs rexafs
and ReFEFF natively; its bundled x64 FEFF10 helper uses [Windows 11
emulation](https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-x86-emulation).
The Windows ARM64 package therefore requires Windows 11. Both Windows
architectures include FEFF10.

### Linux runtime

Use a graphical login session with a Vulkan-capable driver. On Ubuntu 24.04:

```sh
sudo apt-get update
sudo apt-get install -y libgtk-3-0t64 libxkbcommon-x11-0 libvulkan1 \
  libfontconfig1 fonts-dejavu-core xdg-desktop-portal xdg-desktop-portal-gtk
```

Install the Vulkan driver for your graphics hardware. Other distributions require
compatible system libraries. Release GUI checks use X11 and Mesa software rendering;
native Wayland and every physical GPU/monitor configuration are not qualified.

### Verify a download

Each download has a SHA-256 checksum. Compare the downloaded bytes with the linked
checksum file before opening the package. For example:

```sh
# macOS
shasum -a 256 rexafs-0.2.6-aarch64-apple-darwin.dmg
# Linux
sha256sum rexafs-0.2.6-x86_64-unknown-linux-gnu.tar.gz
```

On Windows, use `Get-FileHash -Algorithm SHA256` in PowerShell. The hash should
match the published checksum for that exact file.

## Libraries

- [Python / Jupyter](/docs/libraries/python/): create a uv project, add rexafs, and run Python through that project:

  ```sh
  uv init --python 3.12 rexafs-analysis
  cd rexafs-analysis
  uv add rexafs==0.2.6
  uv run python -c "import rexafs; print(rexafs.__version__)"
  ```

- [TypeScript / JavaScript](/docs/libraries/typescript/): `bun add rexafs@0.2.6`.
- [Rust](/docs/libraries/rust/): `cargo add rexafs@0.2.6`.

Install [uv](https://docs.astral.sh/uv/getting-started/installation/) for the
Python commands or [Bun](https://bun.sh/docs/installation) for the JavaScript
command. The language guides cover alternatives, environment setup and editor
configuration.

Python supports CPython 3.10–3.14; Node requires 22 or newer. Browser use requires
WebAssembly initialization.

Continue with [your first analysis](/docs/getting-started/first-analysis/).
