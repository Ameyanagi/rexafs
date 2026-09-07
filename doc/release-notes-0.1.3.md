# rexafs 0.1.3

Both current FEFF engines are included in the Mac application: ReFEFF 0.3.0 and
FEFF-RS / FEFF10 0.2.3. Choose **Fit → Calculate → engine** to compare calculations
from the same structure and input. ReFEFF remains the default. Each calculation
adds a separate source, labeled by engine, and preserves existing paths and edits.
Select the desired source's paths when comparing fits; enabling both combines them.

ReFEFF now honors the requested calculation timeout through its cooperative
deadline API. The FEFF10 worker hook runs before app argument/GUI initialization.
The package reports its compiled backends and tests actual path calculations after
extraction and installation. Native dependency notices are retained inside the app.

Stable and Nightly Mac releases provide signed, notarized DMG installers and ZIP
archives for Apple Silicon and Intel. The existing updater downloads and verifies
ZIPs for manual installation. Linux and Windows desktop downloads remain withheld
pending graphical qualification; Python, npm and Rust retain their analysis APIs.

Release validation and artifact provenance are recorded alongside the published
downloads. This release does not add manual spectrum reordering or change the
startup Cu example.

For a single-engine comparison, select that source in **Paths**, choose the desired
preset (for example **First shell**), then click **Deselect other sources**. This
retains the other calculations and their parameter edits for later comparison.
