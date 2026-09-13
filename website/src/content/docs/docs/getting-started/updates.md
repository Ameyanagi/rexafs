---
title: "Updates and offline use"
description: "Keep analyses portable and install rexafs without a live connection."
audience: user
---

## Update the desktop

Save the project, then use the action search (Cmd+K on macOS; Ctrl+K on Windows/Linux)
to find **Check for updates**. Stable is the recommended channel for routine work.
Nightly is opt-in and may contain changes that have not reached a stable release.

The Mac updater downloads and verifies an archive; it does not silently replace
the application. Quit rexafs before moving the updated app into Applications.
Nightly has a separate application name so it can coexist with Stable. Windows
users can run the newer installer; portable users replace the complete folder.

Keep a backup when moving between versions. New releases read earlier released
project formats; an older executable may not preserve newer features when saving.

## Install offline

Copy a desktop installer or complete portable folder to another computer with
the same OS and architecture. The bundled Cu example and built-in structures work
offline. Online structure databases and the optional assistant require connectivity.

For Python, use a connected computer with the same OS, architecture and Python
version to download the wheels:

```sh
python -m pip download rexafs==0.2.4 --dest wheelhouse
```

Copy the directory to the offline computer and install:

```sh
python -m pip install --no-index --find-links wheelhouse rexafs==0.2.4
```

For npm, download with `npm pack rexafs@0.2.4`, copy the tarball and run
`npm install ./rexafs-0.2.4.tgz`. For Rust, use `cargo vendor` in your consuming
project and retain its generated Cargo configuration before building offline.

Use an embedded project to move original spectra and FEFF inputs together.
[Project portability](/docs/desktop/projects/) explains what is included.
