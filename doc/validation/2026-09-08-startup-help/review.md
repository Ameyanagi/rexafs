# Startup, Help, and release download polish

This is an unreleased follow-up to 0.2.0, based on `73953ae`. It does not replace
the signed 0.2.0 executables or change the source release tag.

## Behavior

An ordinary launch opens an empty workspace. Empty Groups/parameter panels and
the plot toolbar are hidden. Import and Open project are the primary empty-state
actions; explicit command-line file/project opening keeps the existing route.
Help offers the Cu example, offline licenses, and Updates. The normal top bar
shows an update action only when an update is available.

The license reader includes the application's complete MIT/Apache texts, the
resolved dependency inventory, packaged third-party notices, and example
provenance. License text is loaded locally and only when requested. Mac packages
contain notices inside the app before signing, including unsigned build previews.

GitHub download staging preserves desktop archives, installers, updater assets,
checksums, and installer evidence, while registry packages stay in the build and
registries. Public desktop checksums are distinct from the original complete
build manifest. Release notes put direct platform links first and collapse the
package commands, changelog, and verification details.

## Validation

- GUI release suite: **428 passed**, zero failed, four ignored.
- Release build, extracted-package numerical check, ReFEFF, and FEFF10 checks passed.
- Native packaged Apple Silicon app: empty startup; Help opens the optional Cu
  example and renders its spectrum; MIT, Apache, and a packaged third-party
  license render correctly. Long license text scrolls; selecting another notice
  resets its text scroll. Escape dismisses the reader; stage shortcuts do not
  change the underlying stage while it is open.
- A retained embedded project restored seven groups, two marks, its processing
  lock, pending source, Reference channel, and version-1 recipe. Its disposable
  input bytes remained unchanged.
- License discovery regression covers Mac app-only installation and flat
  Windows/Linux package layouts.
- Two desktop-download tests cover exact copied bytes, manifest preservation,
  excluded registry packages, and rejection of missing/modified/duplicate files,
  wrong versions, and incomplete installer evidence. Staging the real 0.2.0
  artifacts produced 18 unchanged desktop files plus their public manifest.
- Four registry artifact checks and two Mac installer checks pass; actionlint
  and whitespace checks pass.

Local logs: `/tmp/rexafs-startup-help-{check,tests,build,package}.log`.
Local app: `/private/tmp/rexafs-startup-help/target/distributions/rexafs-0.2.0-aarch64-apple-darwin/rexafs.app`.
Native Windows/Linux UI and native Intel hardware were not tested here.

The remaining feature work and broader UI redesign are planning-only additions
to [remaining-work-plan.md](../../remaining-work-plan.md) and
[ui-simplification-plan.md](../../ui-simplification-plan.md).
