# XTUNES test files

These four `.xts` files and two `.xtsp` projects were generated and reopened in
XTUNES 1.3 Build20200228 on Windows. Their original bytes are retained.

Source: [xtunes-analysis, commit e320ed7](https://github.com/Ameyanagi/xtunes-analysis/tree/e320ed77469d848cd0b99125deb810f3f34ccb91/fixtures/xtunes-generated)
(private repository; contains the generation settings, hashes, and verification evidence).

The spectra derive from `PFBL12C_2005.dat` and `PF9A_2022.dat` in the
[xraylarch example collection](https://github.com/xraypy/xraylarch/tree/e3c93284fed358c2c8979cba4c139430527433c6/examples/xafsdata/beamlines),
distributed under the repository's MIT license. The unchanged upstream notice is
retained in [LICENSE.txt](LICENSE.txt); keep it with copied or derived fixtures.
This notice does not license the XTUNES application, which is not included.

- `pfbl12c-bg.xts` and `pfbl12c-ft.xts`: transmission, 818 absorption points.
- `pf9a-bg.xts` and `pf9a-ft.xts`: fluorescence, 1,420 absorption points after
  XTUNES excluded six duplicate energy positions.
- `two-analyzed.xtsp`: both analyzed spectra.
- `mixed-raw-analyzed.xtsp`: an unprocessed transmission record and an analyzed
  fluorescence record.

The `-bg` checkpoints already include XTUNES' automatic Fourier transform.
The `-ft` checkpoints use shorter transform ranges. Embedded Windows paths are
preserved provenance; the numerical data are stored inside each file.
