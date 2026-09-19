# 0.2.12 release qualification

Status: **preparation, not published**. No 0.2.12 tag or public artifact has been
created. Existing 0.2.11 assets remain unchanged.

## Scope

This patch repairs the signed macOS updater helper and keeps the installation
action available after download on all desktop platforms. The
[source qualification](../2026-09-19-updater/review.md) records the reproduced
failure, native tests, full-bundle fix and disposable-app computer-use check.
The source check used an ad hoc signed candidate receiving an official signed
Nightly app; it does not replace final Developer ID qualification.

The screenshot refresh uses the signed 0.2.11 app and public or explicitly
identified academic test inputs. Its capture manifest retains licensing and
scientific scope. No private original measurement project is included.

## Required publication gates

Completed locally: coordinated version validation and its seven regression tests;
new linked/embedded projects written and reopened through the optimized 0.2.12
writer; all 46 retained fixture hashes verified; 29 project tests passed (the
explicit fixture writer remains ignored in the ordinary test run). These checks
do not substitute for the target-specific release matrix.

- Complete the reviewed source and version pull requests into dev, then promote
  dev to main after its selected checks pass.
- Build the immutable version tag using the release workflow; require every
  package, desktop, installer and runtime qualification to pass.
- Sign and notarize both Mac architectures. Check the new helper self-test in
  each signed ZIP and installed DMG, then exercise the signed ARM64 app locally.
- Verify registry package bytes and final desktop asset hashes before announcing
  publication or advancing the website's stable release metadata.

The old Mac helper cannot update itself. Users of affected old versions need
one initial installation of the corrected version; later updates use its repaired
helper. No final signed 0.2.12 update is claimed by this preparation record.
