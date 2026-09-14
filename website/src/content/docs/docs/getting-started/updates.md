---
title: "Updates and offline use"
description: "Keep analyses portable and install rexafs without a live connection."
audience: user
---

## Update the desktop

Save your project, then open **Help → Updates**. You can also search for
**Check for updates** with Cmd+K on macOS or Ctrl+K on Windows/Linux. Use
**Stable** for routine work; **Nightly** is opt-in and includes unreleased changes.

**Check on startup** is enabled by default. Turn it off in this panel for offline
use. Checks do not install software or upload spectra; the channel and startup
preference are saved on this computer.

On macOS, the updater downloads and verifies an archive. Quit rexafs before
moving the updated app into Applications. Nightly has a separate app name and
can coexist with Stable.

On Windows and Linux, use the release link or [download page](/download/). Run the
newer installer or replace the complete portable folder. Built-in verified
downloads support [macOS only in
0.2.4](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/updates.rs#L59).

Cached downloads are verified again before reuse. If a size or SHA-256 check
fails, remove the file named in the error and retry. Failed new downloads discard
their temporary files. After a network error, retry or check the release page;
the error does not establish that your version is current.

Keep a backup when moving between versions. New releases read earlier released
project formats; an older executable may not preserve newer features when saving.

## Install offline

Copy a desktop installer or complete portable folder to another computer with
the same OS and architecture. The bundled Cu example and built-in structures work
offline. Online structure databases and the optional assistant require connectivity.

For Python, prepare uv and the Python interpreter on the offline computer first.
Download wheels on a connected computer with the same OS, architecture and
Python version. Replace 3.12 below with the destination version. Use pip in a
temporary uv environment because uv has no `pip download` command:

```sh
uv run --no-project --python 3.12 --with pip python -m pip download --only-binary=:all: rexafs==0.2.6 --dest wheelhouse
```

On the offline computer, create the project:

```sh
uv init --offline --python 3.12 rexafs-analysis
cd rexafs-analysis
```

Copy `wheelhouse` into the project, then run:

```sh
uv add --offline --no-index --find-links wheelhouse rexafs==0.2.6
uv run --offline python -c "import rexafs; print(rexafs.__version__)"
```

Keep `pyproject.toml`, `uv.lock`, `.python-version` and the wheelhouse to recreate
the environment. These wheels target only the OS, architecture and Python
version used for the download. See [uv's project
workflow](https://docs.astral.sh/uv/guides/projects/).

For TypeScript/JavaScript, install Bun on the destination first. Download the
[published 0.2.6 package archive](https://registry.npmjs.org/rexafs/-/rexafs-0.2.6.tgz)
on the connected computer, copy it to your project on the offline computer, and run:

```sh
bun add ./rexafs-0.2.6.tgz
```

The rexafs 0.2.6 archive includes its WebAssembly binaries and has no runtime
package dependencies. Your application may have other dependencies that need to
be prepared separately. Bun supports [installing local
tarballs](https://bun.sh/docs/pm/cli/add). For Rust, use `cargo vendor` in your
consuming project and retain its generated Cargo configuration before building
offline.

Use an embedded project to move original spectra and FEFF inputs together.
[Project portability](/docs/desktop/projects/) explains what is included.
