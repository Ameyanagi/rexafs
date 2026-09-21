# Copper reduction example (unreleased preview)

`cu-reduction.rxs` contains three measured raw-absorption references and 50
deterministic synthetic mixtures. It is embedded in the desktop executable;
reopening it reuses the same content-addressed input extraction folder. It adds
approximately 564 KiB before installer compression.

The project uses standard automatic desktop processing settings. Fixed
normalization settings in its metadata belong only to a separate numerical
validation and are not applied when the example opens.

The user supplied `Cu oxides.prj` and identified the measurements as their group's
data. The user requested the bundled example and its raw-μ construction in this
development session. The original Athena project remains unchanged and is not
copied here. Its checksum, source labels and synthetic generation parameters are
inside the project. No new acquisition details or upstream data license are
inferred; the software license is not presented as a license for the original
measurements. The older test-fixture permission record is historical.

See [the generation and interpretation guide](../../../../doc/synthetic-copper-reduction.md)
and [`generate-cu-reduction.py`](../../../../scripts/generate-cu-reduction.py).
Generate into a new local directory, validate the results, then copy only the
resulting `Synthetic copper reduction.rxs` here. The user requested local testing
before any push. The historical random-mixture fixtures must not be overwritten.
