# Release 0.2.1 qualification

## Preparation

The release starts from `411a92f32ba1b135d64db48d4a29aa69ba96bac5`, which
contains the merged startup/Help changes, UI and structure improvements, and
ruviz/ruviz-gpui 0.14.1 update. PR #47 passed all 35 checks, and the merged
main Rust run passed. The preparation changes coordinated versions, release
documentation, and the retained-project writer/tests; production behavior is
unchanged from that merged source.

The 0.2.1 writer generated linked and embedded projects from the retained 0.2.0
sample. They preserve the earlier import and Assistant state and add the
AUTOBK/FFT weight link, its independent background value, reversed Viridis group
assignments, and publication style with a mixed Japanese/Latin title. Both were
reopened through the production decoder. All **22** previous sample hashes are
unchanged; the manifest now checks **24** samples.

The complete optimized GUI suite passed **450 tests**, zero failed, five
ignored, with ReFEFF and FEFF10 enabled. The new fixture regression checks that
unlinking restores the independent weight and that palette identities and
publication settings survive reading both storage modes. Existing tests cover
full retained-state round trips and historical defaults.

Coordinated version validation, formatting, whitespace checks, three release
maintenance tests, four registry-artifact tests, and two desktop-download tests
passed. The dependency lock changes only the four workspace package versions.
Local logs: `/private/tmp/rexafs-021-fixture-writer.log` and
`/private/tmp/rexafs-021-gui-tests.log`.

## Publication gates

The tagged manual build, signed downloadable artifacts, available-host checks,
registry publication, and public GitHub release are pending. The release process
uses GitHub-built artifacts and keeps the source tag immutable. Native Intel
hardware, clean-machine installation, and interactive Windows/Linux qualification
remain outside the available host coverage; Windows/Linux retain preview labels.
