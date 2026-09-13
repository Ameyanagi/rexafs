---
title: "Licenses and example data"
description: "Project attribution, packaged notices and measurement provenance."
audience: user
---

## rexafs

rexafs is available under the [MIT license](/LICENSE-MIT.txt) or
[Apache License 2.0](/LICENSE-APACHE.txt), at your option. Copyrights remain with
its contributors. Dependencies and embedded calculation engines retain their
own licenses and notices; see **Help → Licenses** in the packaged application.

## Cu example

[Download cu_150k.xmu](/examples/cu_150k.xmu). The original file contains a Cu foil
measurement at 150 K from NSLS X-11A, September 1992. It is retained without
numerical changes from the XrayLarch example collection at revision
`d8678dd666fd95839fe9dc71b4dbe8bedec278ff`. The header also identifies its UWXAFS
3.0 distribution history. Retain that header when redistributing the example.

[Source and provenance](https://github.com/xraypy/xraylarch/blob/d8678dd666fd95839fe9dc71b4dbe8bedec278ff/examples/xafsdata/cu_150k.xmu)
· [Retained provenance record](/examples/PROVENANCE.txt).

## Documentation screenshots

The desktop guides show full, unedited application-window captures from the
published macOS ARM64 0.2.4 package. They were captured through computer use on
13 September 2026 with the Cu example and built-in Cu structure. The fitting
walkthrough uses ReFEFF, an 8 Å cluster and one first-shell path. Values in these
screenshots describe that demonstration, not a benchmark or universal fit result.

## Scientific citations

Use the [references and citation guide](/docs/science/references/) for algorithm
references. Cite the actual measurement, structure source and FEFF backend used
in your analysis in addition to the software version.
