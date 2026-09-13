# Windows installers

Published Windows 0.2.4 includes a setup EXE alongside the portable ZIP:
`rexafs-VERSION-x86_64-pc-windows-msvc-setup.exe`.
Download it from the [GitHub release](https://github.com/Ameyanagi/rexafs/releases),
run it, and launch **rexafs** from the Start menu. A desktop shortcut is optional.

That installer supports x64-compatible Windows 10 version 2004 or newer and Windows
11. Its executable is x64. It installs for the
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
Updating those private DLLs requires an updated rexafs package.

The v0.1.3 installer contains the same application EXE and resources as the
already-published Windows ZIP. It includes ReFEFF 0.3.0; the FEFF10 prebuilt
uses MinGW and could not be linked into that MSVC build. Windows bundles built
from the current source add FEFF10 as the bundled `resources\feff10\feff10-rs.exe`
helper process with its MinGW runtime DLLs and notices; the installer copies
the complete bundle tree, so no installer change is needed. Codex CLI is a
separate optional installation for the assistant. Packaging this release does
not add unreleased assistant or other GUI changes.

## ARM64 in the next-release pipeline

The source checkout also builds `aarch64-pc-windows-msvc` ZIPs and setup EXEs
on the native `windows-11-arm` runner. These packages are not published or
qualified yet. The ARM64 installer requires Windows 11 ARM64 with x64 emulation:
rexafs and ReFEFF are native ARM64, while the bundled FEFF10 helper remains x64.
[Microsoft's emulation documentation](https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-x86-emulation)
explains why Windows 10 ARM64 is insufficient.

Packaging checks the desktop's PE machine type and selects matching Microsoft
runtime DLLs. New portable ZIPs include these signed DLLs and record their hashes
and signatures in `build.json`; installers preserve and revalidate that payload.
The ARM64 installer uses Inno Setup's `arm64 and x64compatible`
admission rule. Its metadata records native desktop architecture and the helper's
emulation requirement. The installed checks must run on native ARM64 Windows;
an x64 test under emulation cannot qualify the ARM64 executable. The two stable
architectures use the same product identity and install directory, so switching
architecture replaces the installed application rather than creating a second copy.

## Build and verify

The installer uses [Inno Setup](https://jrsoftware.org/isinfo.php). On a Windows
machine with Inno Setup 6.6 or newer, Python 3.12, uv and Visual Studio C++ tools:

```powershell
uv run --no-project python scripts/windows_installer.py target/distributions --bundle "target/distributions/rexafs-*-pc-windows-msvc"
```

The builder infers the target from `build.json`. Add
`--target aarch64-pc-windows-msvc` to require an ARM64 bundle explicitly; a mismatch
is an error. The target's C++ runtime redistributables must be installed with
Visual Studio. Native ARM64 jobs use the ARM64 redistributable directory, not x64
DLLs from a tool running under emulation.

To wrap an existing release's qualified Windows ZIP without rebuilding the app:

```powershell
uv run --no-project python scripts/windows_installer.py windows-installer --release v0.1.3
```

The second command uses `gh` to download the ZIP and checksum from
`Ameyanagi/rexafs`, then checks the version, channel, target, clean build flag and
source commit against the requested GitHub tag before compiling an installer.
It checks archive paths before extraction. The payload is copied into temporary
staging; the source ZIP/bundle is not changed. For older ZIPs without bundled
Microsoft runtime metadata, the builder adds DLLs from the matching Visual Studio
redistributable directory after validating their architecture and Microsoft
signatures. New ZIPs retain their recorded runtime files; a changed hash or invalid
signature is an error. Each setup EXE has a `.sha256` and a `.build.json` sidecar recording
the source build, installer compiler, runtime versions/signatures, and every
installed payload hash.

Release downloads default to x64 for compatibility with existing releases.
`--target aarch64-pc-windows-msvc` requests the ARM64 ZIP, but only works once
that exact release has published it. The standalone workflow exposes the same
target choice and selects a matching native runner; it does not create a missing
release archive.

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
