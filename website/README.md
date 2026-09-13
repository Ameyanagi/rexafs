# Website maintenance

This directory builds the public [rexafs website](https://rexafs.com/)
with Astro and Starlight. This README is for maintainers and is not bundled.
The [audience inventory](../doc/documentation-audience.csv) and
[site plan](../doc/documentation-site-plan.md) explain the content boundary.

## Build and preview

Use Node 24, Python 3.12 and Rust 1.98.1. From the repository root:

```sh
npm --prefix website ci
python3.12 -m venv website/.venv
website/.venv/bin/python -m pip install rexafs==0.2.4 numpy
website/.venv/bin/python website/scripts/generate-python-reference.py
node website/scripts/generate-typescript-reference.mjs
node website/scripts/generate-citations.mjs
node website/scripts/build-rust-reference.mjs
npm --prefix website run check
npm --prefix website run build
npm --prefix website test
npm --prefix website run preview
```

Open the preview's `/rexafs/` path. `npm --prefix website run dev` gives live
editing; generate Rust reference files first if you want their links to work.
The Rust generator downloads the published crate, verifies its recorded SHA-256,
and runs rustdoc with all features and no dependency documentation. Cargo may
download optional engine build artifacts. Existing warnings in the published
crate's rustdoc remain visible in the build log; the website does not patch a
release's source silently. Set `DOCS_CARGO_TARGET_DIR` to an absolute cache path
to reuse compilation between local builds.

For browser checks, run these commands inside `website/`:

```sh
npx playwright install chromium
npm run test:browser
```

Tests exercise links and anchors at the deployment base, user-only content,
math rendering, citation links, original screenshot dimensions, generated API
coverage, code tabs, search, mobile overflow and automated accessibility checks.
Browser screenshots are retained in `test-results/` for visual review. Automated
checks supplement review of the prose, figures, source claims and citations.

## Content ownership

- `src/content/docs/` owns the curated public manual. Every page requires
  `audience: user`; the collection schema rejects any other audience.
- `doc/` retains source-checkout guides and historical records. Website guides
  target published 0.2.4; they intentionally differ from checkout guides that
  document unreleased APIs. Keep shared scientific explanations synchronized
  when the underlying method changes. Do not import `doc/` recursively.
- `src/content/docs/docs/reference/{stable,next}/` is generated. Edit Python
  stubs/native docstrings or TypeScript declarations, then regenerate. Stable
  uses the release tag and installed wheel; Next uses checkout declarations.
  Next pages are visibly labeled and excluded from the default search index.
- `public/api/rust/` is generated from the checksum-verified published crate and
  ignored by Git. It includes public modules enabled by optional features.
- `src/data/api-citations.json` is generated from authored API documentation
  links, including scientific papers, specifications and supporting code. New
  links must still be reviewed for relevance and explained where they are used.
- `public/` contains only selected public assets. Application screenshots are
  full, unedited 1192 × 768 JPEG captures made with computer use from the official
  macOS ARM64 0.2.4 release. Keep their version, input and capture provenance in
  the public licenses page. Do not crop them or replace plots with mockups.

The writing and scientific baseline in [CONTRIBUTING.md](../CONTRIBUTING.md)
applies to every page, example and generated source comment. XrayLarch's official
manual informed the reader journeys; scientific behavior must be traced to
rexafs's own source and described with appropriate citations.

## Update a release

Update `src/data/release.json` only after desktop assets, PyPI, npm and crates.io
are actually published. Verify every asset and checksum URL. Record the exact tag,
commit and crate checksum. Install the matching Python wheel, update versioned
installation examples and the workflow pin, regenerate all API pages, and review
which Next changes have become stable. Regenerate screenshots when their workflow
changes; retain their actual software version in captions. Run the documented
Python and Node Cu examples with the corresponding published packages.

The Website workflow regenerates references and fails on drift before building.
Pull requests build and test; only `main` deploys to the `github-pages` environment.
The Pages publishing source must be **GitHub Actions**. No generated `dist/` or
`gh-pages` branch is committed.

## Domain and deployment paths

The repository's Pages custom domain is `rexafs.com`; its Actions variables are
`SITE_URL=https://rexafs.com` and `SITE_BASE=/`. DNS and certificate provisioning
must complete before the custom address is reachable. The domain uses Cloudflare
nameservers. Set the four apex A records from
[GitHub's custom-domain guide](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site)
and `www` CNAME to `ameyanagi.github.io`, using DNS-only records during setup.
Preserve unrelated DNS records, including mail and verification records. Enable
**Enforce HTTPS** once GitHub's certificate is ready.

These two build variables change navigation, assets, canonical URLs, sitemap and
search together. The fallback without variables is the original project path,
`https://ameyanagi.github.io/rexafs/`. No `CNAME` file is required for a custom
GitHub Actions deployment. If reverting to the project address, remove the Pages
custom domain and unset the two variables before redeploying.

Test the domain's root-path build locally:

```sh
SITE_URL=https://rexafs.com SITE_BASE=/ npm --prefix website run build
SITE_URL=https://rexafs.com SITE_BASE=/ npm --prefix website test
SITE_URL=https://rexafs.com SITE_BASE=/ npm --prefix website run test:browser
```

Restart any existing preview server when switching base paths. After deployment,
check downloads, a deep manual link, search, a full screenshot and Rust reference
on the custom domain, and verify the old Pages address redirects as intended.
Consult the [Astro GitHub Pages guide](https://docs.astro.build/en/guides/deploy/github/)
for hosting behavior.
