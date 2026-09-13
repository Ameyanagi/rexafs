# Website maintenance

This directory builds the public [rexafs website](https://rexafs.com/)
with Astro and Starlight. This README is for maintainers and is not bundled.
The [audience inventory](../doc/documentation-audience.csv) and
[site plan](../doc/documentation-site-plan.md) explain the content boundary.

## Build and preview

Use Node 24, Python 3.12 and Rust 1.98.1. From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked --version 0.15.0
npm --prefix website ci
uv run --no-project --python 3.12 website/scripts/generate-python-reference.py
node website/scripts/generate-typescript-reference.mjs
node website/scripts/generate-citations.mjs
node website/scripts/build-rust-reference.mjs
npm --prefix website run check
npm --prefix website run test:generators
npm --prefix website run build
npm --prefix website test
npm --prefix website run preview
```

The Python generator parses source declarations with the standard library; it
does not import rexafs or require a package installation. `--no-project` avoids
building the repository's Python package just to render its documentation.
User analysis projects use the [uv init/add/run workflow](src/content/docs/docs/libraries/python.md).

`package.json` overrides every nested `katex` dependency to the top-level
`katex` version so the rendered math HTML and the bundled `katex.min.css` come
from the same release; KaTeX 0.18 renamed its sizing classes, and a mismatch
leaves subscripts and superscripts unstyled. After changing `katex` or the
Markdown plugins, run `npx astro build --force`: Astro's content cache otherwise
keeps HTML rendered by the previous version. The browser test checks that a
subscript is rendered smaller than its base.

Open the preview's `/rexafs/` path. `npm --prefix website run dev` gives live
editing; generate Rust reference files first if you want their links to work.
The Rust generator downloads the published crate, verifies its recorded SHA-256,
and runs rustdoc with the default backend and optional capabilities, with no
dependency documentation. It deliberately excludes `ndarray-compat`, which
replaces several default numerical modules with legacy implementations. Cargo may
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
Generator tests use temporary release tags and edited source docstrings to check
that explanations propagate, unreleased signatures stay out of Stable, and
removing a public member's help fails generation.

## Browser workspace

`/app/` runs spectrum processing locally in a module Worker, using this checkout's
WASM bindings. It is labeled as an unreleased preview. It supports numeric
text/CSV import, explicit column roles and energy units, normalization, AUTOBK,
forward Fourier transforms and CSV/JSON export.

`/app/scattering/` runs the published ReFEFF 0.4.0 WASI engine through its browser
Worker. It accepts a FEFF input, displays calculated EXAFS and offers generated
files and a provenance record for download. Native rexafs 0.2.5 retains its
ReFEFF 0.3.0 dependency; the two browser engines have separate manifests.

`npm run dev` and `npm run build` first prepare both engines. Install `wasm-pack 0.15.0`
and the `wasm32-unknown-unknown` target before either command. Cargo's binary
directory must be on `PATH`; `REXAFS_WASM_PACK` can select an explicit executable.
The build stages only browser runtime assets in ignored `public/wasm/` and writes
a manifest with the source commit, uncommitted-change flag and WASM SHA-256. The Worker verifies that hash
before initialization; no source files are uploaded for processing.

Import/setting limits are documented in `src/browser/input.ts` and tested with
`npm test`. After normalization resolves E0, the preview checks that the inferred
AUTOBK grid fits its unchanged default of 2048 points at 0.05 Å⁻¹ spacing. Both
forward-transform grid choices must fit the calculated k grid and upper window
extent within the selected FFT length; insufficient lengths produce an error
instead of silent truncation. A preview-specific budget also limits each
estimated dense spline basis to 2 million double-precision values (16 MB).
The estimate multiplies the larger of the post-edge raw/grid counts by AUTOBK's
automatic knot count, including its 5–128 knot bounds. It does not bound total
browser memory. These guards reject input without cropping or resampling it;
they do not change the core numerical API. See the implementing
[AUTOBK grid](../crates/rexafs/src/xafs/background.rs) and
[basis allocation](../crates/rexafs/src/xafs/background/fixed.rs).

Browser tests process the real Cu fixture, compare exports with Node,
check cancellation/recovery and verify that edited settings invalidate exports.
Cancel terminates the Worker and discards its state; progress reports stage
boundaries. The JSON export records requested settings and resolved E0; other
inferred settings are not exposed by the current bindings.

ReFEFF assets come from the pinned upstream release archive, verified by SHA-256
during staging. The build retains engine, adapter and third-party notices in
ignored `public/refeff/`. No ReFEFF source build is needed for this website.
Staging uses Python 3's standard ZIP reader; `REXAFS_PYTHON` can select it.
`REXAFS_REFEFF_ARCHIVE` accepts a local copy of the pinned archive for offline
builds without bypassing checksum verification.
The browser verifies the WASM bytes before starting a calculation and loads
them only on demand. Its tests compare all ZnSe `chi.dat`/`xmu.dat` numeric
columns with retained serial native 0.4.0 output at the upstream tolerance,
`5e-8 + 5e-5 * abs(reference)`. That case does not qualify every FEFF workflow.
The unchanged upstream input includes a Kr scatterer; preserve this distinction
when describing the example.

`vendor/refeff-0.4.0/` retains the Cargo dependency notices and source inventory.
Ordinary builds verify and copy these files. When updating the pinned engine,
follow the [notice generation instructions](vendor/refeff-0.4.0/README.md) and
update the staging hashes with the reviewed outputs. The Python 3.12 generator
reads the pinned upstream commit without modifying its checkout or rebuilding
WebAssembly. From the repository root, verify the retained outputs with:

```sh
uv run --no-project --python 3.12 website/scripts/generate-refeff-notices.py \
  --refeff-repository ../refeff --check
```

The same directory retains runtime notices from the identified Rust distribution
and its WASI libc sources. Their inventory records exact source URLs, hashes and
retained byte ranges. They are refreshed separately from the Cargo generator;
follow the vendor README and preserve the original notice bytes.

## Content ownership

- `src/content/docs/` owns the curated public manual. Every page requires
  `audience: user`; the collection schema rejects any other audience.
- `doc/` retains source-checkout guides and historical records. Website guides
  target published 0.2.5; they intentionally differ from checkout guides that
  document unreleased APIs. Keep shared scientific explanations synchronized
  when the underlying method changes. Do not import `doc/` recursively.
- `src/content/docs/docs/reference/{stable,next}/` is generated. Edit Python
  stubs/native docstrings or TypeScript declarations, then regenerate. Stable
  membership and signatures come from the release tag; Next uses the checkout.
  Both use maintained checkout docstrings/JSDoc for explanations. Help for shared
  members must be checked against both implementations: do not describe a new
  calling convention as available in the stable release. Put those examples in
  the Next guide. Generators reject missing descriptions instead of rendering
  empty fields or Python's generic constructor help. Next pages are visibly
  labeled and excluded from the default search index.
- `public/api/rust/` is generated from the checksum-verified published crate and
  ignored by Git. It includes public modules enabled by optional features while
  preserving the default numerical backend.
- `public/api/rust-next/` is generated separately from the checkout, with its
  updated Rust comments and API. Its banner states that it is unreleased and
  links back to Stable. Do not use it as evidence of a released signature.
  Missing public Rust documentation and broken intra-doc links fail the Next
  build. Generated HTML is cleared between channels while compilation caches
  are reused, so obsolete pages cannot carry over into the release reference.
- `src/data/api-citations.json` is generated from authored API documentation
  links, including scientific papers, specifications and supporting code. New
  links must still be reviewed for relevance and explained where they are used.
  Extraction includes the Python declarations/native help, every TypeScript
  entry-point declaration, Wasm help and the Rust core's public documentation.
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
commit and crate checksum. Install the matching Python wheel in a separate uv
analysis project to verify the examples, update versioned installation examples,
regenerate all API pages, and review
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
nameservers. The configuration verified on 2026-09-13 has both `rexafs.com` and
`www.rexafs.com` as proxied CNAME records targeting `ameyanagi.github.io`, with
automatic TTL. Cloudflare supports the apex record through
[CNAME flattening](https://developers.cloudflare.com/dns/cname-flattening/).
The browser connects to Cloudflare; successful public HTTPS checks alone do not
verify the separate Cloudflare-to-GitHub TLS settings. The `www` address redirects
to `https://rexafs.com/`.

If intentionally moving away from Cloudflare's proxy, follow
[GitHub's custom-domain guide](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site),
verify GitHub's certificate for the custom hostname, then enable **Enforce HTTPS**.
Preserve unrelated mail and verification records in either configuration.

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
