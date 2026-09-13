# User documentation and rexafs.com plan

Status: implemented in `website/`, 13 September 2026. This developer planning
record is not included in the public website. See the
[maintenance guide](../website/README.md) for the actual build, content ownership
and future custom-domain steps. The sections below preserve the design rationale.

The initial migration retains checkout guides because they describe unreleased
APIs, while curated website guides target stable 0.2.4. Shared science text must
be synchronized on method changes; it is not imported from mixed developer pages.
API pages and their authored reference links are generated. Twelve full desktop
window captures, taken through computer use, illustrate the public workflows.

## Recommendation

Build one website in this repository: a small product homepage, a download page,
and a searchable user manual. Publish desktop workflows, library usage and
scientific explanations. Keep contributor instructions, implementation plans,
release machinery, audits and research archives in GitHub.

The homepage gives **Desktop** and **Libraries** equal prominence, as requested
on 13 September 2026. Present two equally weighted entry points: desktop analysis
and library usage in Python, TypeScript and Rust. Both should be immediately
discoverable from the main navigation and the opening section of the homepage.

A programmer using rexafs is a **user**. API examples, installation, editor setup,
configuration defaults and error handling belong on the website. A programmer
changing rexafs itself needs **developer documentation**: source architecture,
test/fixture maintenance, packaging, CI and release instructions.

## Inventory and classification

The [file-by-file inventory](documentation-audience.csv) classifies all **100
tracked prose documents at commit `6cb668d`**: Markdown, MDX, reStructuredText and
extensionless `README` files. There are 6 user documents, 26 mixed documents and
68 developer/research records. These are source-file counts, not planned page
counts or a certification that every sentence is ready for publication.
Later additions are appended to the inventory; the counts above remain the
historical snapshot at that commit.

The inventory specifies each file's audience, migration action, proposed public
route and editing notes. It is a planning catalog, not a website publishing
configuration. This plan and the CSV are additional developer-only artifacts.
License/notice files, embedded API comments and non-prose assets follow the rules
below even though they are outside the 100-file prose count.

| Class | Existing sources | Website treatment |
|---|---|---|
| User | `doc/api.md`, `doc/processing-theory.md`, `doc/fitting-statistics.md`, `doc/publication.md`, `doc/publication-export.md`, `crates/rexafs/README.md` | Adapt into current, versioned user pages; merge overlapping topics. |
| Mixed: entry points and installation | Root/Python/TypeScript READMEs; `doc/installing.md`, `macos-installers.md`, `windows-installers.md`, `desktop-updates.md`, `migration.md` | Extract installation and usage; retain contributor builds, packaging and CI details in developer docs. |
| Mixed: desktop | `doc/xdi-import.md`, `joint-fitting.md`, `project-compatibility.md`, `structure-depth-view.md`, `experimental-assistant.md` | Keep user actions and observable behavior; separate schema internals, protocol implementation and dated qualification records. |
| Mixed: scientific reference | `doc/autobk-fixed-penalty.md`, `fft-grid-compatibility.md` | Keep equations, parameter effects and compatibility guidance; separate solver/cache engineering and validation commands. |
| Mixed: releases and attribution | `CHANGELOG.md`, `COPYRIGHT.md`, published `doc/release-notes-*.md` | Create user-facing release history and an attribution page; preserve original notices and repository records. |
| Developer | `CONTRIBUTING.md`, `AGENTS.md`, release/dependency/development runbooks, design plans, fixture/test READMEs, internal assistant instructions | Exclude from website output and search. |
| Developer/research archive | `doc/validation/`, `doc/benchmarks/`, `doc/plots/`, profiling notes, `experiments/`, both `supportinginfo/` trees, historical core design notes | Preserve in GitHub; exclude from launch content and assets. |

Mixed is a migration state, not a third public audience. Each mixed document must
be split into user content and developer content before its user portion is
published. Installation from source can still be useful to an advanced user;
keep a short verified recipe where needed, while contributor setup and package
production remain separate.

### Specific cleanup decisions

- `doc/macos-installers.md`: publish its installation/update instructions;
  retain **Bundling and qualification** and **Local preview** in developer docs.
- `doc/windows-installers.md`: publish installation, removal, compatibility and
  current signature/backend limits; retain **Build and verify** and historical
  packaging details in developer docs.
- `doc/project-compatibility.md`: publish sharing, supported formats, size limits
  and recovery. Separate detailed JSON layout and **Every-release regression record**.
- `doc/joint-fitting.md`: publish **Workflow** and **Independent batches**;
  retain **Validation, 2026-09-06** and its audit screenshots in the repository.
- `doc/experimental-assistant.md`: publish optional setup, controls, permissions,
  shared context and saved-conversation behavior. Retain its final module/protocol
  implementation paragraph in developer docs. Disclosure of data sent through the
  assistant is user information and must remain visible.
- Combine `doc/publication.md` and `doc/publication-export.md` into one coherent
  guide, with export-file details as a reference section.
- `doc/release-notes-0.2.2.md` is a withheld qualification attempt with unpublished
  download URLs. Exclude it from published-release navigation and downloads.
- The older `crates/rexafs/supportinginfo/uncertainty*.md` duplicates contain
  unresolved statements. Use the current processing/statistics guides for public
  explanations; do not copy either teaching-note tree automatically.
- The normalization experiment is a prototype, not the shipped normalization
  algorithm. Its interactive HTML and generated data stay outside the launch site.

## Public navigation

The homepage should explain the shared scientific purpose, followed by two
equally prominent panels with matching heading levels and primary-button styles:

- **Desktop:** an actual application screenshot, a short workflow description,
  **Download the desktop** and a link to the first-analysis guide.
- **Libraries:** a concise processing example with Python, TypeScript and Rust
  tabs, **Use the libraries** and links to installation and API guides.

Place the panels side by side on wide screens and stack them with equal visual
weight on narrow screens. Give **Desktop** and **Libraries** sibling positions in
the main navigation. Scientific explanations and troubleshooting serve both
audiences. Use the existing brand artwork and label scientific figures with their
data source.

| Route | Purpose |
|---|---|
| `/` | Product overview, supported workflows and entry points. |
| `/download/` | Current stable desktop assets by OS/architecture; package-install links. |
| `/docs/getting-started/` | Installation, first analysis, updates and offline use. |
| `/docs/desktop/` | Import, groups, processing, structures/paths, fitting, projects and publication. |
| `/docs/libraries/` | Python, TypeScript/JavaScript and Rust quick starts; common spectrum API. |
| `/docs/science/` | Processing equations, AUTOBK, Fourier conventions and fitting statistics. |
| `/docs/troubleshooting/` | Installation/import errors, unavailable outputs, editor environment and recovery. |
| `/releases/` | Published changes and user migration notes. |
| `/licenses/` | Project attribution, relevant notices and sample-data provenance. |

Within Libraries, separate Python from TypeScript and provide clear Node/browser
examples. Show which operations are available in each binding; do not imply that
the smaller Python/TypeScript API already exposes Rust's fitting/structure APIs.
Link Rust users to rustdoc for the selected crate release.

Science belongs in the user manual. Each workflow should give a short explanation
of what it calculates, link to the detailed theory, and state useful defaults.
Readers should not need a developer runbook to understand a result.

### Missing user pages to write

The current documentation is stronger on implementation records than on complete
beginner workflows. These need new task-oriented pages, verified in the selected
release rather than copied from design proposals:

1. First analysis: open the bundled Cu example, inspect energy/absorption, normalize,
   remove the background, transform, save a project and export a figure.
2. General text import: transmission/fluorescence, column mapping, energy units,
   import recipes, groups and per-spectrum overrides. XDI alone is insufficient.
3. First structural fit: choose/import a structure, select the absorber and backend,
   calculate/select paths, set parameters/ranges, fit and inspect residuals.
4. Troubleshooting: invalid energy arrays, missing prerequisites/results, Python
   interpreter selection, browser initialization, missing source files and recovery.
5. Citation guidance: how to cite rexafs, the actual algorithms, FEFF backend and
   input data. Do not invent a software-paper citation or experimental provenance.

Include one small, redistributable example dataset with its retained attribution.
Use it consistently in the desktop and applicable library tutorials. Make the
example downloadable so code blocks do not depend on private filesystem paths.

## Website implementation

Use **Astro with Starlight** and a small custom Astro homepage. Starlight accepts
Markdown/MDX with page metadata and provides documentation navigation; ordinary
prose can remain Markdown. Its default Pagefind search works on the built static
site. This matches a manual with code examples and scientific reference pages.
See [Starlight authoring](https://starlight.astro.build/guides/authoring-content/)
and [site search](https://starlight.astro.build/guides/site-search/).

Configure math rendering explicitly using `remark-math` and `rehype-katex`,
including the KaTeX stylesheet. Verify the existing equation syntax in rendered
pages. These plugins parse mathematical notation and render it as HTML; they do
not verify the scientific content. See the
[official math-plugin project](https://github.com/remarkjs/remark-math) and
[Astro Markdown configuration](https://docs.astro.build/en/guides/markdown-content/).

Use Starlight's normal public-content directory to avoid a custom documentation
loader or a second independently maintained copy of the manual:

```text
website/
  src/pages/                 Product homepage and download page
  src/content/docs/          Canonical public guides, science and release pages
  src/assets/                Selected, attributed screenshots and diagrams
  public/                    Explicitly selected static files only
  astro.config.mjs
  package.json
  package-lock.json
doc/
  README.md                  Repository navigation by audience
  dev/                       Active maintainer guides after migration
  validation/                Existing historical evidence; never bundled
  benchmarks/                Existing historical evidence; never bundled
  plots/                     Existing historical evidence; never bundled
experiments/                 Research prototypes; never bundled
```

Configure public paths to match the route table, independently of old filenames.
During migration, move each curated guide once and replace its old repository path
with a short link to the canonical source/page. Update links and preserve useful
old heading anchors where practical. Keep package READMEs as concise installation
and quick-start entry points that link to the detailed manual. Preserve historical
links in immutable release tags and link evidence by commit/tag when relevant.

Do not recursively import `doc/` or the repository into the site. Only reviewed
public source files and selected assets enter the build. Developer files must be
absent from generated HTML, downloadable assets, sitemap and search index; hiding
them from the sidebar or adding `noindex` does not satisfy that requirement.

The repository remains public, so intentional links to GitHub source or scientific
evidence are fine. They should support an explanation, not supply missing user
instructions. This separation controls website content, not repository access.

## Versions and API reference

Stable documentation must match installable packages and released desktop builds.
The current README explicitly marks the new keyword/options constructors and
`XrayFFTR` binding support as additions after 0.2.4. Merging those changes into main
does not itself publish them to PyPI, npm or crates.io.

For the first site release, choose a verified published target. Either document
that release's API or publish the binding improvements through the normal package
release process before making them the stable tutorial. An unreleased source-build
example must be labeled accordingly and must not sit beneath a registry-install
command that produces a different API.

Keep a small checked-in release manifest with the selected versions and validated
asset links. Refresh it during release preparation, using actual GitHub release
assets and package versions. The website deploy must not publish packages. Fail
validation on draft/withheld releases or missing download assets. Avoid constructing
asset names from assumptions or querying GitHub on every visitor's page load.

Start with one stable manual and a dated release history. Add a separate, clearly
labeled next-version manual only when it is useful; keep it out of default stable
search results. A full version selector for every historical patch is unnecessary
at launch.

Preserve Python docstrings/stubs and TypeScript declarations/JSDoc as the source
for editor-visible API contracts. Reuse the installed-package editor tests and
execute website examples against the declared package versions. The launch site
can use focused reference pages for the small exposed surface and link versioned
Rust rustdoc. Do not maintain a second unchecked list of default values.

A later API-generation step can extract TypeScript declarations and Python
installed-wheel signatures/docstrings. Prototype one configuration class first,
including optional values, defaults and hover text; PyO3 native classes and `.pyi`
files need explicit verification before selecting a Python generator. Generation
must supplement human-written tutorials and theory, and must use the same release
as the rest of the page.

## Deployment to rexafs.com

Default recommendation: **GitHub Pages with GitHub Actions**, since the source and
release workflow are already in GitHub and this is a static site. Astro documents
this deployment path, including custom domains. Keep `site` as `https://rexafs.com`
when serving the apex domain, without a repository-path prefix. See the
[official deployment guide](https://docs.astro.build/en/guides/deploy/github/).

The repository has no website scaffold or `.openai/hosting.json` in the inspected
checkout. The authenticated Pages lookup returned HTTP 404, and HTTPS requests to
rexafs.com timed out from this environment. This does not establish the current
DNS ownership or hosting provider. Verify those settings before changing them.

Implement deployment in this order:

1. Build and review the static artifact locally and in PR checks. Use local preview
   or CI artifacts first; hosted PR previews are an optional later addition.
2. Configure Pages and verify control of the custom domain. Inspect existing DNS
   first, including any mail records or existing service at the domain.
3. Set the apex domain and, if desired, `www` redirect using the provider's current
   instructions; verify HTTPS and canonical URLs. Follow
   [GitHub's custom-domain guide](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site).
4. Deploy the exact reviewed build through a dedicated workflow. Limit routine
   triggers to website/user-document changes and relevant release metadata.
5. Check live downloads, search, equations, mobile navigation and links. Retain the
   previous successful artifact/commit so a failed website deployment can be rolled
   back without changing the application or package releases.

If rexafs.com already uses another static host, the same generated artifact can be
used there. Confirm that existing setup before choosing a migration. This plan
makes no DNS, hosting, package-release or deployment changes.

## Work sequence and completion criteria

| Change | Deliverable | Completion criterion |
|---|---|---|
| 1. Separate and edit content | Curated user pages; developer remainders; updated repository/package links | Every migrated file has one canonical owner; no mixed page is copied unchanged. |
| 2. Build the site | Homepage, navigation, search, math, downloads and release metadata | Static build works; user pages and selected assets are the only output. |
| 3. Complete user journeys | First analysis/fit/import guides, screenshots, examples and troubleshooting | A new user can install, analyze, save, export and run a library example using only website instructions. |
| 4. Qualify and launch | Checks, custom domain, deployment and rollback record | Reviewed artifact serves correctly over HTTPS at rexafs.com. |

Apply [CONTRIBUTING.md](../CONTRIBUTING.md) throughout: clear English, defined
symbols/units, explained assumptions/defaults, verified citations and an honest
distinction between implemented behavior and proposed or historical work.

Before launch, verify:

- The website build, internal links, anchors, download targets and code examples.
- Math, code blocks, tables, screenshots and navigation in light/dark themes and
  narrow viewports; include keyboard navigation and readable figure descriptions.
- Public API examples against the selected Python/TypeScript packages, including
  the existing completion, hover and signature checks when API help changes.
- Search finds installation, E0, `rbkg`, FFT, Python and TypeScript, and cannot find
  developer-only titles or withheld/unreleased download pages.
- Generated files, assets and search indexes contain no developer/research pages.
- Scientific references support the adjacent claims and defaults match the selected
  implementation. Automated builds do not replace this review.

A release checklist should then update the release manifest, user-facing changes,
examples and affected theory/API pages together. Website prose corrections can
ship independently when they still describe the declared software version.
