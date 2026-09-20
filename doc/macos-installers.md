# macOS installers

Download the `.dmg` for **Apple Silicon** (`aarch64`) from the release page. From 0.2.12, desktop and Python distributions no longer support Intel Macs. The archived [0.2.11 release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.11) retains its Intel downloads. Open it, drag rexafs onto **Applications**, eject the image, then open rexafs from Applications. Save the project and quit the older app before replacing it. `rexafs Nightly.app` has a separate name and bundle identifier so both channels can coexist.

The app contains its examples and license notices; copying the app is sufficient. Notices are under **Show Package Contents → Contents/Resources/notices**. Project files stay separate. The updater uses a verified ZIP and offers **Update and restart** on supported installations; see the [updater qualification and migration guidance](desktop-updates.md).

## Bundling and qualification

`sign-macos-release.py --dmg` takes an existing, checksum-qualified GitHub build. It copies the build's notices inside the app before signing, signs with Developer ID Application and Hardened Runtime, notarizes and staples the app, then creates both a ZIP and a drag-to-Applications DMG. The binary is not rebuilt. The DMG itself is signed with Developer ID Application, notarized and stapled.

The signing job checks the outer DMG with `codesign`, `stapler`, and Gatekeeper's `open` assessment. It mounts the image read-only, checks the app's signature and Gatekeeper `execute` assessment, and copies the app to a fresh temporary installation. The installed copy must preserve the signature, source/channel identity and architecture, and pass the packaged spectrum self-check. The job never overwrites an existing installation or project.

Each installer has `.dmg.sha256` and `.dmg.json` sidecars. The JSON records the app build/signing runs and source commits, both notarization submissions, installer digest, related ZIP digest, executable digest, and successful installation checks. The Nightly publisher requires the Apple Silicon package in both formats, and verifies the uploaded digests before making the draft public. It does not replace a public Nightly or change the latest Stable release.

The reviewed Stable signing workflow produces these artifacts for maintainer qualification. Adding installers to an existing release must preserve its public ZIPs, package artifacts, checksums and tag. In that case publish a separate `INSTALLER-SHA256SUMS` and retain the signing-run link: the accompanying ZIP in the signing run is a new packaging artifact, not a replacement for the existing public ZIP.

## Local preview

On a Mac, create a dedicated uv project for the packaging tools. Replace the
repository and extracted-build paths below with your local paths:

```sh
uv init --python 3.12 rexafs-installer-tools
cd rexafs-installer-tools
uv add -r /path/to/rexafs/scripts/macos-installer-requirements.txt
uv run python /path/to/rexafs/scripts/preview-macos-installer.py /path/to/extracted/rexafs-VERSION-TARGET ./installer-preview
open ./installer-preview/*-preview.dmg
```

`uv add` records the dependencies in `pyproject.toml` and `uv.lock`; `uv run`
uses the project's environment without manual activation. The input is the
packaged directory containing `build.json`, notices and the `.app`. A preview
modifies only a temporary copy, uses an ad hoc app signature, and is visibly
named `-preview.dmg`. It is **unsigned and not notarized**, and is never a release
installer. The release-build pull request job exercises this preview on Apple
Silicon without signing credentials; signed installers are produced only by the
reviewed Stable or dev-branch Nightly workflows.

`dmgbuild` and its two dependencies are pinned in `scripts/macos-installer-requirements.txt`. They are packaging tools and are not embedded in the app. The layout uses the app's existing icon and Finder's system background, which respects light/dark appearance.
