# Windows installers

Windows releases include a setup EXE alongside the portable ZIP:
`rexafs-VERSION-x86_64-pc-windows-msvc-setup.exe`.
Download it from the [GitHub release](https://github.com/Ameyanagi/rexafs/releases),
run it, and launch **rexafs** from the Start menu. A desktop shortcut is optional.

The installer supports x64-compatible Windows 10 version 2004 or newer and Windows
11. The executable is x64; it is not a native ARM64 build. It installs for the
current user under `%LOCALAPPDATA%\Programs\rexafs`, without requesting
administrator rights. Remove it through Windows **Settings → Apps**.

Save your work and close rexafs before installing an update. Installing another
stable version uses the same installation directory and uninstall entry. The
installer does not force-close a running app. It preserves user-created files
on uninstall and does not delete `.rxs` projects or the user's `.rexafs` settings.
The portable ZIP remains available for users who prefer manual installation.

Source builds use the Windows GUI subsystem, so opening the app from Explorer
or a shortcut does not create a terminal window. Diagnostic flags still support
redirected output. When scripting these flags in PowerShell, use
`Start-Process -Wait` with `-RedirectStandardOutput` and `-RedirectStandardError`
to wait for the GUI executable and collect its output reliably.

The setup EXE and rexafs EXE currently have no Windows publisher signature.
The release provides SHA-256 checksums and installer qualification records.
The included Microsoft runtime DLLs retain Microsoft's signatures. Microsoft
Visual C++ runtime files are installed beside rexafs, using
[local deployment](https://learn.microsoft.com/en-us/cpp/windows/deployment-in-visual-cpp).
Updating those private DLLs requires an updated rexafs installer.

The v0.1.3 installer contains the same application EXE and resources as the
already-published Windows ZIP. It includes ReFEFF 0.3.0; the FEFF10 prebuilt
currently uses MinGW and cannot be linked into this MSVC build. Codex CLI is a
separate optional installation for the assistant. Packaging this release does
not add unreleased assistant or other GUI changes.

## Build and verify

The installer uses [Inno Setup](https://jrsoftware.org/isinfo.php). On a Windows
machine with Inno Setup 6.7.1, Python 3.12, uv and Visual Studio C++ tools:

```powershell
uv run --no-project python scripts/windows_installer.py target/distributions --bundle "target/distributions/rexafs-*-pc-windows-msvc"
```

To wrap an existing release's qualified Windows ZIP without rebuilding the app:

```powershell
uv run --no-project python scripts/windows_installer.py windows-installer --release v0.1.3
```

The second command uses `gh` to download the ZIP and checksum from
`Ameyanagi/rexafs`, then checks the version, channel, target, clean build flag and
source commit against the requested GitHub tag before compiling an installer.
It checks archive paths before extraction. The payload is copied into temporary
staging; the source ZIP/bundle is not changed. Microsoft runtime DLLs are accepted
only from the Visual Studio redistributable directory with valid Microsoft
signatures. Each setup EXE has a `.sha256` and a `.build.json` sidecar recording
the source build, installer compiler, runtime versions/signatures, and every
installed payload hash.

`.github/workflows/windows-installer.yml` builds and tests a wrapper for an
existing release; it does not publish automatically. The regular release build
also creates and qualifies an installer for every Windows desktop build, and
the reviewed-release publisher includes it in the draft release and checksum
manifest.

Windows CI runs `scripts/test-windows-installer.ps1`: silent install, comparison
of every installed payload hash, shortcut and per-user uninstall registration
checks, reinstall, installed `--build-info`, `--self-check`, `--self-check-feff`,
uninstall, and verification that a user-created project survives. It installs
into a temporary path containing spaces and Japanese characters. These checks
run only on an ephemeral GitHub Actions runner, and do not replace interactive
GUI testing on a user's Windows machine.

The installer definition keeps stable and nightly application names, directories,
shortcuts and uninstall identifiers separate. The current scheduled nightly
workflow builds macOS; Windows nightly publication is not enabled by this change.
