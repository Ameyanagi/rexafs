# Development, nightly builds and stable releases

Use `dev` as the development integration branch and `main` as the stable branch.
Create each feature or research branch from `dev` and open its pull request
against `dev`. For example:

```sh
git fetch origin
git switch -c feature/xanes-components origin/dev
gh pr create --base dev
```

Keep unfinished research and implementation on their feature branches. A research
proposal does not authorize implementing its proposed scientific behavior.

## Nightly channel

A push to `dev` starts the Nightly desktop workflow. Maintainers can also run:

```sh
gh workflow run nightly.yml --ref dev
```

The daily schedule is stored on `main`, GitHub's default branch. That small job
dispatches `nightly.yml` on `dev`; it does not build the main-branch application.
GitHub assigns the dispatched run its own immutable source commit. Core checks,
both Mac builds, signing and publication all use that same run/commit identity.
Moving `dev` during a build cannot change its checked-out source. Reruns preserve
the original source and dated release identity.

GitHub allows `workflow_dispatch` to start another workflow using the repository
workflow token; see [GitHub's trigger documentation](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).
The dispatcher has Actions write permission for that operation. Build jobs use
read access, and only the publication job can write release assets.

The `macos-signing` and `nightly` environments admit the `dev` branch. The stable
signing workflow still requires `main`, and the registry publication environment
continues to require version tags. Nightly publication rejects main, feature
branches, pull-request refs and mismatched source identities. No credential
values need to be copied between branches.

Nightly currently publishes signed/notarized Mac ARM64 and Intel apps and DMGs.
It does not publish nightly Rust, Python or npm packages. `rexafs Nightly.app`
coexists with the stable application; Nightly is an opt-in update channel.
Windows and Linux remain covered by the pull-request/release qualification
matrix and are not newly advertised as nightly downloads.

## Promote a release

1. Merge completed feature pull requests into `dev`. Squash merges are suitable
   for these short-lived branches.
2. Prepare the coordinated version, release notes and compatibility fixtures on
   `dev`; finish the required checks and review the nightly workflow.
3. Open a release pull request **from `dev` to `main`**. Review the complete
   release difference and wait for its selected checks.
4. Merge that long-lived branch with a **merge commit** so Git retains the
   shared branch history. Do not squash or rebase the `dev` → `main` promotion.
5. Create an immutable version tag on the merged main commit. Follow the
   [release runbook](releasing.md) to build, qualify, sign and publish it.
   A nightly artifact does not replace the required manually dispatched build
   of the exact release tag.
6. Merge `main` back into `dev` so subsequent release pull requests start from
   the published state. Keep pending feature branches separate until ready.

Rust and release checks accept pull requests to both branches. Website checks
also run for development changes, while the public website deploys only from
`main`. Dependabot targets `dev`, so dependency updates join the same review and
release process. The default GitHub branch remains `main`.

## Main branch protection

Protection verified on 16 September 2026 requires pull requests to `main` to be
up to date and pass the GitHub Actions **Rust checks** and **Release checks**
aggregate jobs. Review conversations must be resolved. Force pushes and branch
deletion are blocked; these rules also apply to administrators. Merge commits
remain available for the release workflow. No independent approval is required,
so a sole maintainer can merge a qualified release.

The two aggregate jobs always run and validate every check selected for the
change, including an intentional documentation-only selection. Keep their names
stable when editing CI. Path-filtered jobs must not become required checks unless
they are changed to report a result for every pull request; otherwise an absent
job can block unrelated changes indefinitely.

If a release pull request is behind `main`, merge `main` into `dev` through the
pull request's **Update branch** action, then wait for checks on the updated
head. Do not bypass protection or force-push the long-lived branches.

## September 2026 transition

The old `dev` branch had eight commits from the earlier xraytsubaki development
line and had diverged from the current project. Its exact tip,
`080e678f3641d1c60aba2e1ca36997eafb2e2b37`, is preserved as
`archive/dev-before-nightly-2026-09-14`. Merge
`0ee3b953f4fddf9fc814e46cc569f40f634f250f` retains that ancestry while adopting
exactly the current stable source tree at
`8ba48385ca00273bd8bd2bf7e2c46d291e6e5ad1`. No force-push or history deletion
is needed to advance `dev` from its earlier tip.

Version 0.2.6 was already prepared and tagged before this branch-policy change.
Its tag and release build remain unchanged. The initial `dev` → `main` pull
request establishes the workflow configuration; later release promotions use
the process above.
