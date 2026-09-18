# rexafs 0.2.10 qualification record

Prepared on 18 September 2026. **Not yet published.** This record distinguishes
source preparation from immutable-tag build, signing and public-artifact checks.

## Source and scope

The release starts from `dev` commit
`4d211b973297b23dbbf7baaa6787720322374cdd` (PR 89), incorporating PRs 85–89.
Main's existing promotion commit is merged back before preparation to retain
shared branch history. The [release notes](../../release-notes-0.2.10.md) describe
RMC, analysis, processing, Series and experimental Live workflows.

The preceding RMC integration passed 48 CI checks. On its combined source,
457 core tests and 618 desktop tests passed locally, along with strict core
Clippy and an optimized build. Native macOS accessibility interaction opened the
saved 5,396-attempt Cu₂O checkpoint and the current Cauchy Wavelet workspace.
Those checks identify the preceding development source, not this future tag.
The Cu₂O residual remained StillChanging; no converged refinement is claimed.

## Release preparation

Coordinated Cargo/npm versions advance to 0.2.10; Python inherits Cargo's version.
API source documentation identifies the introduced version. Website Stable
metadata stays on 0.2.9 until all public distributions are verified.
The maintainer writer produces new linked and embedded format-1 fixtures without
rewriting historical files. The maintainer writer passed; all 29 project tests
passed, including loading, saving and reopening every retained release fixture.
The fixture manifest validates 42 samples. Coordinated version, formatting and
whitespace checks passed. Python/TypeScript references and citations were
regenerated; all eight documentation-generator tests passed. Stable Rustdoc
was restored from its verified published-crate cache, Next Rustdoc built
successfully, and the website source check reported zero errors or warnings.

## Remaining publication gates

1. Review the dev-to-main release PR and pass its current-head checks.
2. Merge with a merge commit, create an immutable v0.2.10 tag and manually build
   that exact tag through Release builds.
3. Verify original artifact hashes, publish the qualified Rust/Python/npm files,
   sign and notarize both original Mac archives, and qualify the extracted apps.
4. Replace draft Mac downloads with signed outputs, verify the final desktop
   manifest and publish the GitHub release.
5. Verify public package/download bytes, promote Stable references and install
   links, and merge main back into dev.

Windows/Linux desktop and Live acquisition limits remain explicit. Native macOS
qualification does not establish behavior on physical Windows/Linux graphics,
network shares or every experimental dataset. Existing adaptive RMC remains
experimental and opt-in.
