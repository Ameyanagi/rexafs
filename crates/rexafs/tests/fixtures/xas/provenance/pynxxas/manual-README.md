These examples ilustrate early attempts to convert "raw" or "as collected" beamline data
into HDF5/NeXuS NXxas groups, and also into "new style" XDI files that map directly to
the NXxas groups.

There files here are essentially built "by hand", using a "convert_to_nexus.py" script
in each folder.  This is meant to be a proof-of-principle, and to allow for
discussion about these formats.  These examples are also meant to drive the
development of pynxxas and NXxas definition.


For all datasets:

1. "intensity" is edge-step normalized XAS.  The processing steps are
    done with xraylarch, but in a way (linear pre-edge, constant edge step),
    that it should be easy to reproduce.

2. "energy" is strictly in eV.

3. "New" XDI files are written to very, very closely match the NXxas Groups,
   though they do not have the full range of "hierarchy" or automated setting
   of attributes. An important goal is that data can be converted between these
   two forms without any loss of data or important metadata.

4. The group "rawdata" holds a 2-d array (Ncolumns, Nenergy) with all data that
   maps to the columns of data from the original data file. Usually, these will
   be identical to the original data.  But note that some files (Fe_XDIFiles)
   had "mufluor" as a column, and the new files have converted these to
   "ifluor": The rawdata also includes (in column 2) the "intensity" or
   "normalized mu(E)".  That is, the data shape is identical to the output
   XDI file.

The "convert_to_nexus.py" scripts are not very sophisticated and are not meant for real
use in pynnxas, but more of a guide of what operations and steps to do.  Also,
the intention is to *not* rely on xraylarch, but to port code to robustly read
text files, and do basic processing (including pre-edge subtraction and
normalization) into pynxxas.

Folders and Files (so far):
===========================

Fe_XDIFiles:  3 XDI files (all Fe K edge, transmission XAFS)
      are put into a single nexus file.

KEK_PF: 1 data file from Photon Factory, KEK.  Fluorescence XAFS,
      that was recorded as a function of angle

MultiElementFluor:  12 data files (XDI-ish) for V K-edge fluorescence XAFS
      using a 4-element detector.  The original files have columns saved
      for "V Ka" and a "Dead-time Correction Factor" for MCAs 1 through 4.
      The script sums the deadtime-corrected channels to make "ifluor".
      V_XANES_nexus.h5

BlissMultiModal:  HDF5 EXAFS data at the As K-edge in transmission and fluorescene.
      Multiple EXAFS channels are recorded at the same time.
      - energy_enc: energy in keV
      - p201_1_bkg_sub: I0
      - p201_3_bkg_sub: I1
      - p201_5_bkg_sub: I2
      - xmapd16_det<i>: MCA with "roi1" as the fluorescence signal of interest
