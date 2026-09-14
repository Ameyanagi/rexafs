This collection has no blanket license. Each measurement retains the license
or usage notice identified in `manifest.json` and its adjacent `.license` file.
The full texts and original upstream notices are in [LICENSES](LICENSES/).
No measurement bytes were edited, normalized, or converted during collection.

The `samples/` subset contains 137 files with documented license terms:

| Basis | Files | Terms retained |
| --- | ---: | --- |
| Explicit public-data grant | 3 | NIST BMM standards; unchanged NIST notice and acknowledgment retained |
| Explicit data dedication | 11 | CC0-1.0, XASDataLibrary |
| Explicit dataset or spectrum license | 45 | CC-BY-4.0, MDR and CLS XASDB |
| Explicit dataset license | 19 | CC-BY-NC-SA-4.0, MDR |
| Repository distribution license | 46 | MIT, Larch, ParSeq-XAS, xasref and pynxxas |
| Repository distribution license | 13 | Artistic-1.0-Perl, Demeter |

The repository-license entries rely on the upstream license covering bundled
examples/test fixtures; a separate creator-issued data license was not located.
That basis is recorded distinctly from an explicit data license. Existing
copyright notices remain in the original files and upstream license texts.
Demeter offers the same terms as Perl; this collection records its supplied
Artistic license option.

[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) requires attribution,
a license link and disclosure of changes when redistributing.
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) additionally
restricts commercial use and requires adaptations to retain the same license.
Use the noncommercial files for the requested noncommercial tests and preserve
their notices; they cannot be relicensed as part of an MIT-only data bundle.
The actual license texts govern. Merely choosing a noncommercial software
license does not itself determine whether a particular use is noncommercial.

Twelve examples in `candidates/refxas/` were obtained from RefXAS. Their source
links, citations and original
[RefXAS usage notice](LICENSES/LicenseRef-RefXAS-Usage-Notice.txt) are retained.
`LicenseRef-RefXAS-Usage-Notice` identifies that notice; no standard license is
assigned to these files.

[CITATIONS.md](CITATIONS.md) and the manifest provide attribution and provenance
for every measurement. Repository revisions and checksums identify the exact
copies used. No license for future rexafs implementation code is selected here.

The additional ESRF BM16 BLISS example is distributed under the pinned pynxxas
repository's MIT notice, retained in
[LICENSES/XraySpectroscopy--pynxxas--LICENSE](LICENSES/XraySpectroscopy--pynxxas--LICENSE).
This is a repository distribution license for the bundled example, not an
inferred facility-wide data license. The source sidecar and manifest retain
credit, exact download URL and checksum.

Five additional [KEK Photon Factory QD measurements](candidates/kek-pf/README.md)
are retained for academic, nonmilitary research and reader regression testing
only. Their [original usage notice](LICENSES/LicenseRef-KEK-PF-Academic-Use-Notice.txt)
permits academic research excluding military-related research and requests
contact with the named experimenter for publication citation. The custom
`LicenseRef-KEK-PF-Academic-Use-Notice` identifier does not assign a standard
license or assert broader permissions. Original source pages, attribution and
unchanged measurement bytes are retained together. These fixtures are excluded
from crates.io, Python and npm packages.
