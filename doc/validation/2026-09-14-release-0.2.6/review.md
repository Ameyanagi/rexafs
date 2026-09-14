# rexafs 0.2.6 release qualification

Status: preparation in progress. No 0.2.6 registry packages, signed installers or
public desktop release are claimed by this record yet.

The shared reader was squash-merged in
[PR #64](https://github.com/Ameyanagi/rexafs/pull/64) at
`6d2f53ab96f98d97c06b3b4fdd0b91c62bb71cf8`. All checks on its final revision
passed, including Windows fixture checks, benchmarks and six desktop targets.
Those PR artifacts carry 0.2.5 metadata and must not be published as 0.2.6.

## Source preparation

Cargo's workspace and local lockfile entries, the workspace rexafs dependency,
and npm's manifest/lockfile advance together to 0.2.6. Python derives its version
from Cargo. Reader API help identifies 0.2.6 as the introduction version. Website
Stable metadata stays at 0.2.5 until actual publication is verified.

Generate and reopen linked and embedded projects through the 0.2.6 writer;
preserve all historical project and beamline fixture bytes. Review the release
notes, generated Next API help, package exclusions and coordinated-version check.

## Release gates

1. Complete preparation checks and merge the reviewed release commit.
2. Create an immutable `v0.2.6` tag, then manually dispatch the full release
   build on that exact tag. Only its successful, checksummed artifacts may be
   promoted; local builds and PR artifacts remain verification outputs.
3. Qualify the four ABI3 wheels on all 20 interpreter/platform combinations,
   the source archives, npm package and all six native desktop targets.
4. Sign and notarize both qualified Mac archives, verify the resulting ZIPs and
   DMGs, and replace unsigned Mac assets in the draft desktop release.
5. Publish GitHub-built packages to crates.io, PyPI and npm. Verify public bytes
   and fresh consumer installations, then publish the complete desktop release.
6. Promote website downloads and Stable references to the verified tag and crate
   checksum. Check the deployed site and preserve versioned screenshot evidence.

The [release runbook](../../releasing.md) defines the commands and platform
limits. Numerical defaults and project format remain unchanged; qualification
does not establish experimental accuracy or compatibility with every GPU.

## Local preparation evidence

On macOS ARM64, the 0.2.6 maintainer writer saved and reopened both new project
fixtures. The focused project suite passed 26 tests with the explicit fixture
writer ignored; all 34 retained project/source samples passed their checksum and
header checks. Historical fixture hashes were preserved.

The coordinated `v0.2.6` version check and strict Rustdoc passed. Python and
TypeScript Next references were regenerated with 130 and 182 documented members;
Stable remains at the unchanged 0.2.5 declarations. Release CI and publication
remain separate gates, as listed above.
