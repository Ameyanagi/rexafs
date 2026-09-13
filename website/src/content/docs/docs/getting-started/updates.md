---
title: "Updates and offline use"
description: "Keep analyses portable and install rexafs without a live connection."
audience: user
---

## Update the desktop

Save the project, then use the action search (Cmd+K on macOS; Ctrl+K on Windows/Linux)
to find **Check for updates**. Stable is the recommended channel for routine work.
Nightly is opt-in and may contain changes that have not reached a stable release.

**Help → Updates** opens the same panel. **Check on startup** is enabled by
default and can be turned off here; checks do not install software or upload
spectra. The update channel and startup preference are saved on this computer.

The Mac updater downloads and verifies an archive; it does not silently replace
the application. Quit rexafs before moving the updated app into Applications.
Nightly has a separate application name so it can coexist with Stable. Windows
users can run the newer installer; portable users replace the complete folder.

The built-in verified-download action is available for macOS in 0.2.4. On Windows
or Linux, follow the release link and choose the matching package from the
[download page](/download/). This distinction comes from the supported targets in
the [update client](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/updates.rs#L59).

Keep a backup when moving between versions. New releases read earlier released
project formats; an older executable may not preserve newer features when saving.

## Install offline

Copy a desktop installer or complete portable folder to another computer with
the same OS and architecture. The bundled Cu example and built-in structures work
offline. Online structure databases and the optional assistant require connectivity.

For Python, prepare uv and the Python interpreter on the offline computer first.
Use a connected computer with the same OS, architecture and Python version to
download wheels. This example uses Python 3.12; change it to the version on the
destination. uv does not provide a `pip download` subcommand, so use pip in a
temporary uv-managed environment for this preparation step:

```sh
uv run --no-project --python 3.12 --with pip python -m pip download --only-binary=:all: rexafs==0.2.4 --dest wheelhouse
```

Copy the directory to the offline computer, then create an environment and
install from the local wheels:

```sh
uv venv --offline --python 3.12
uv pip install --offline --no-index --find-links wheelhouse rexafs==0.2.4
```

The `--offline` option prevents network access; creating the environment requires
the interpreter to be installed already. See [uv package
installation](https://docs.astral.sh/uv/pip/packages/).

For TypeScript/JavaScript, install Bun on the destination first. Download the
[published 0.2.4 package archive](https://registry.npmjs.org/rexafs/-/rexafs-0.2.4.tgz)
on the connected computer, copy it to your project on the offline computer, and run:

```sh
bun add ./rexafs-0.2.4.tgz
```

The rexafs 0.2.4 archive includes its WebAssembly binaries and has no runtime
package dependencies. Your application may have other dependencies that need to
be prepared separately. Bun supports [installing local
tarballs](https://bun.sh/docs/pm/cli/add). For Rust, use `cargo vendor` in your
consuming project and retain its generated Cargo configuration before building
offline.

Use an embedded project to move original spectra and FEFF inputs together.
[Project portability](/docs/desktop/projects/) explains what is included.
