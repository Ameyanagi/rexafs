# Desktop updates and release channels

Open **Updates** in the top bar, or search for **Check for updates** with Cmd+K.

**Stable** is the default for routine analysis. **Nightly** is opt-in and follows daily builds of main. The selected channel and the **Check for updates on startup** preference belong to this computer, not the project. Automatic checks do not install software or upload spectra.

Choose **Download** to fetch the matching Mac archive and verify its size and SHA-256. **Show download in Finder** reveals the completed ZIP. Save the project, quit the app, extract the archive, and move the application into Applications. Nightly is named `rexafs Nightly.app`, so it can coexist with Stable. Choosing Stable from a Nightly app explicitly offers the stable release, even if its library version is older.

A Nightly label includes the immutable build tag. `rexafs --build-info` reports the library version, channel, release tag, source commit and optional nightly build time. Each packaged archive contains the same identity and signing/notarization provenance in `build.json`.

## Updating to 0.2.0

Save the project before updating. Version 0.2.0 adds persistent group identities,
marks, labels, locks, import recipes and applications, pending sources, and parser
evidence to format-1 projects. Existing mappings remain authoritative on reopen;
computer recipes apply only to new imports. Original source files are unchanged.
Keep the previous-save backup when moving an ongoing analysis to this version.

## Updating to 0.1.4

Save the project before quitting for an update. Version 0.1.4 adds optional
Assistant conversations to format-1 projects; older projects begin with an empty
history. The Assistant's host, width, model preferences and conversation-save
limit belong to this computer. Conversation contents belong to the project and
are written on Save. The default limit is five; zero disables saving. Existing
previous-save backups remain available.

## Maintainer operation

`.github/workflows/nightly.yml` runs at 18:23 UTC daily and supports manual dispatch on main. It does not run publication from pull requests or other branches. A date plus GitHub run ID forms each tag; published nightlies are never overwritten and are never marked as the latest stable release.

The workflow builds macOS ARM and Intel packages with ReFEFF, runs core and desktop checks, and uses the existing `macos-signing` environment for Developer ID signing and Apple notarization. No signing secrets are available in build jobs. It produces ZIP archives and [drag-to-Applications DMG installers](macos-installers.md), with installation checks and notices inside the app. The final `nightly` environment publishes only after both formats and architectures pass source, target, checksum and notarization-provenance checks. Uploaded GitHub digests are checked before the draft becomes public. A failed draft can be resumed by rerunning the workflow; an already-public nightly requires a new dispatch/run ID.

The separate reviewed stable workflow remains `publish.yml`; its crates.io, PyPI and npm trusted publishers are unchanged. Windows/Linux portable desktop previews and a Windows setup installer are available for platform testing. Native interactive qualification remains pending; automated archive and installation checks are recorded with the releases.
