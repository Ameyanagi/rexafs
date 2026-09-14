# rexafs 0.2.7 release qualification

Status: preparation in progress; no 0.2.7 packages or desktop assets are published.

The release combines the Codex startup correction in
[PR #70](https://github.com/Ameyanagi/rexafs/pull/70), the Assistant composer and
panel layout in [PR #71](https://github.com/Ameyanagi/rexafs/pull/71), and the
measurement reader corrections in [PR #72](https://github.com/Ameyanagi/rexafs/pull/72).
All three features passed their selected checks before merging into `dev`: 47
checks for #70, 47 for #71, and 48 for #72. The website deploy job was intentionally
skipped on each PR. Their benchmark logs reported no threshold regressions.

The merged `dev` tree at `8378ebac724e1ffb7d3e925d7ee97b52f79e85e6` exactly
matches the combined source used for the local tests below. The release promotion
uses a `dev` → `main` pull request with a merge commit.

## Qualification procedure

1. Coordinate Cargo and npm versions; Python derives its version from Cargo.
   Save and reopen new linked and embedded compatibility fixtures with the
   0.2.7 writer, preserving historical files and their checksums.
2. Review the release changes and promote `dev` to `main`. Create the immutable
   `v0.2.7` tag and manually dispatch the complete release build for that tag.
3. Require all package, interpreter and desktop jobs to pass. Publish only
   checksummed artifacts from that exact GitHub build.
4. Sign and notarize both Mac archives, qualify the resulting ZIPs and DMGs,
   and inspect the actual Assistant interface with computer use in an isolated
   test session. Capture versioned screenshots for the updated guide.
5. Publish crates.io, PyPI and npm packages and verify public bytes and fresh
   consumers. Complete and verify the desktop draft before making it public.
6. Update the public download metadata and Stable API references only after
   publication, verify the deployed guide and retain screenshot provenance.

The [release runbook](../../releasing.md) defines the commands and platform
limits. Passing numerical tests does not establish experimental accuracy, and
archive self-checks do not replace graphical review.

## Local preparation evidence

On Apple Silicon macOS, the combined source from PRs #70–72 passed
`cargo test --locked -p rexafs -p rexafs-gui`: **821 passed, 0 failed, 10 ignored**.
This includes 478 GUI tests and the core suites with the GUI's enabled features.
The combined check used 0.2.6 metadata before the coordinated version change.

After the version change, the 0.2.7 maintainer writer saved and reopened new
linked and embedded project fixtures. The project suite passed **26 tests**,
with its explicit fixture writer ignored. All **36 retained samples** passed
checksum and header verification; historical file bytes and hashes are unchanged.
The coordinated `v0.2.7` version check, formatting, and Rustdoc with
missing-documentation and broken-link errors enabled passed. Python and
TypeScript references regenerated with 130 and 182 documented members per channel;
Stable continues to describe published 0.2.6 until publication is verified.

These local checks do not qualify a release tag, public packages or signed
desktop downloads. Their results will be supplemented by the exact-tag build
and graphical review below.

## Local Assistant interface review

Computer use exercised an optimized 0.2.7 review app on Apple Silicon macOS,
with a separate settings file and the bundled public Cu example. It connected
to the installed Codex model catalog with `PATH=/usr/bin:/bin`. No assistant
message was sent. The following workflows passed visual and accessibility checks:

- Model, Reasoning and Access menus display their current choices and descriptions.
- Escape, Tab, Enter and Space navigate and activate menus once.
- Parameters closes and reopens with `⌘J` while a composer control has focus.
- A widened Assistant makes room for the most recently opened Groups or Parameters
  panel; normalization parameters remain visible and usable.
- Pop out retains the composer and model catalog, its menu stays within the
  separate window, and Dock returns the same session to the analysis.

The unedited local review capture and receipt are under
`/tmp/rexafs-release-0.2.7/gui-candidate/`. This is a source-build review; final
public screenshots must come from the qualified signed release download.
