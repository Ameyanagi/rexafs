# Measurement import UI review

Captured through computer use on 14 September 2026 from the unreleased macOS
ARM64 source build. The [capture record](capture.json) identifies the executable,
inputs and full 1187 × 768 JPEG screenshots by SHA-256. No images were cropped,
resized, composited or redrawn. This is visual/workflow evidence, not numerical
or cross-platform qualification.

- [EX3 preview](../../../website/public/screenshots/next/import-preview.jpg):
  624 points with detected energy and stored absorption selected automatically.
- [Original source header](../../../website/public/screenshots/next/import-source-details.jpg):
  source details expand below the plot while the import action remains visible.
- [Imported raw absorption](imported-ex3.jpg): one spectrum is added; Undo removes
  it and the toolbar Redo action restores it. The keyboard Redo chord did not
  change the observed tree during this check; toolbar Redo was verified.
- [KEK QD angle preview](kek-qd-preview.jpg): 3,917 points, transmission, and the
  recorded 3.13551 Å crystal-plane spacing selected for Bragg conversion.

The EX3 input is Masashi Ishii and the Industrial Application and Partnership
Division's [XAFS spectrum of Lead telluride](https://doi.org/10.48505/nims.3178),
retained under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
The original measurement bytes are unchanged; the screenshots visualize the
stored energy and absorption without additional import corrections.

The KEK image is academic, nonmilitary reader-validation evidence using Yasuhiro
Inada's LiFePO4 measurement, [PF record 132](https://pfxafs.kek.jp/xafsdata/view.php?id=132).
The [original usage notice and experimenter attribution](../../../crates/rexafs/tests/fixtures/xas/candidates/kek-pf/README.md)
apply. This image is retained with repository validation records; the public
Next guide uses the separately attributed EX3 screenshots.

Released-version desktop screenshots remain unchanged. The Next guide and
public licenses page identify these new captures as unreleased.
