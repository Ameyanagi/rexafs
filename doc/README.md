# rexafs documentation

The [documentation audience inventory](documentation-audience.csv) classifies
every existing prose document as user, developer, or mixed content. The
[rexafs.com plan](documentation-site-plan.md) explains how to split mixed guides
and publish only user-facing content. These two planning files are developer
documents. The curated [public manual](https://rexafs.com/docs/getting-started/)
is built from `website/src/content/docs/`; see the
[website maintenance guide](../website/README.md) for generation and deployment.
The manual follows the [verified release metadata](../website/src/data/release.json),
while source-checkout guides can describe clearly labeled unreleased additions.

Start with the [project README](../README.md), [API guide](api.md),
[Rust guide](../crates/rexafs/README.md), [Python guide](../py-rexafs/README.md) or
[JavaScript guide](../js-rexafs/README.md).

## Support rexafs

If rexafs is useful to you, [star the repository on GitHub](https://github.com/Ameyanagi/rexafs)
or [sponsor development through GitHub Sponsors](https://github.com/sponsors/Ameyanagi).
Sponsorship supports ongoing development and maintenance; stars help others discover the project.

## Scientific explanations and documentation standards

- [How processing works](processing-theory.md): transmission, normalization,
  AUTOBK, Fourier transforms and filtering, with equations and references.
- [Fitting statistics](fitting-statistics.md): residuals, information counts,
  covariance, standard errors and interpretation limits.
- [Experimental RMC with ReFEFF](rmc.md): unreleased atomic-coordinate refinement,
  Rust examples, resumable mixtures, evolutionary search and scientific limitations;
  [measured acceleration and validation](rmc-performance.md),
  [prepared paths and current Rust additions](rmc-acceleration.md),
  [hybrid search and adaptive basis stages](rmc-search-upgrade.md),
  [auditing and exact fallback](rmc-adaptive-audits.md),
  [Spectrum input and preprocessing snapshots](rmc-spectrum-input.md),
  [Cu₂O adaptive qualification](rmc-cu2o-adaptive-qualification.md), and an
  [experimental Cu₂O example with k/R plots](rmc-cu2o-demo.md).
- [Fixed-penalty AUTOBK](autobk-fixed-penalty.md): the rexafs-specific objective.
- [Contributor documentation baseline](../CONTRIBUTING.md): requirements for
  clear English, defined symbols/units, verified citations and useful API help.
- [September 13 source/documentation audit](documentation-audit-2026-09-13.md):
  developer record of reviewed areas, corrections, checks and scope limits.
- [Documentation and API priorities](documentation-api-roadmap.md): concise
  website ownership, binding improvements and ARM64 distribution work.
- [WebAssembly assessment](webassembly.md): browser processing and ReFEFF 0.4.0,
  historical compile probes and remaining portability work.

## Release and migration

- [Rebranding plan](rebranding-plan.md): decisions, implementation order and release gates.
- [Migration guide](migration.md): imports, packages and existing desktop data.
- [Release runbook](releasing.md): builds, verification and publication.
- [Dependency update record](dependencies.md): latest stable Rust and compatibility constraints.
- [Distribution license review](distribution-notices.md): observed dependency terms and remaining release work.

## Desktop workflows

- [Windows installation and packaging](windows-installers.md)
- [Linux and Windows development and repository hooks](desktop-development.md)
- [Universal measurement reader](measurement-reader.md): unreleased beamline, HDF5 and XTUNES support.
- [Larix session import](larix-import.md): stored absorption, complex arrays and inert session metadata; [implementation plan](larix-import-plan.md).
- [Demeter and Larch format audit](reference-format-audit.md): pinned sources, new original fixtures and qualified reader behavior.
- [Source repository research](beamline-source-repositories.md): pinned GitHub sources and fixture license evidence.
- [XDI import](xdi-import.md)
- [Multiple spectra and independent fitting](joint-fitting.md)
- [Series browsing and trends](series.md)
- [Structure slices and depth cues](structure-depth-view.md)
- [Project compatibility and recovery](project-compatibility.md)
- [Publication editor and captions](publication.md)
- [Publication export](publication-export.md)
- [Experimental assistant](experimental-assistant.md)
- [Desktop workflow validation](gui-workflow-validation.md)

## Design and scientific history

These records describe the implementation at their stated dates. Validation counts
and benchmark timings are historical, not assertions about the current build.

- [Desktop UI/UX review of 0.2.11 (2026-09-19)](ux-review-0.2.11.md): proposals with retained captures
- [Fitting workspace redesign](fitting-workspace-redesign.md)
- [Structure database design](structure-db-design.md)
- [GUI design](gui-ux-design.md) and [v2 proposal](gui-ux-design-v2.md)
- [Original FEFF fitting scope](../crates/rexafs/doc/feff-fitting-mvp.md)
- [Performance and logic migration](../crates/rexafs/doc/migration-performance-logic-hardening.md)
- [Profiling](profiling.md) and [extended core profiling](../crates/rexafs/doc/profiling.md)
- [Larch/rexafs benchmark matrices, output agreement and CPU profiles (2026-09-06)](benchmarks/2026-09-06-larch/README.md)
- [FEFF/Larch comparisons](plots/feff_vs_larch_index.md)
- [FEFF10 card comparison](plots/feff10_card_comparison_2026-03-03/report.md)
- [Uncertainty notes](../supportinginfo/uncertainty.md) and [additional notes](../supportinginfo/uncertainty2.md)
- [Larch fixture generation](../crates/rexafs/tests/pythonscript/README.md) and [fit reference provenance](../crates/rexafs/tests/testfiles/larch_fit_refs/README.md)

## Historical numerical research

- [September 7–8 processing benchmarks](benchmarks/2026-09-07-08-summary.md): rexafs 0.1.3 methodology, aggregate measurements and limitations.
- [Normalization stability prototype](../experiments/normalization_stability/README.md): reproducible comparison of four models on Ru/Cu fixtures.

The [reader refactor plan](measurement-reader-refactor-plan.md) records the
fixture, test and adapter organization and the desktop preview follow-up.
