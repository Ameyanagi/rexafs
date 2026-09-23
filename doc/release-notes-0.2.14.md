# rexafs 0.2.14

Published on 24 September 2026 in Japan (23 September UTC), with Live monitoring,
EXAFS parameter trends and Assistant file import. The tagged build, signed Mac
artifacts, installed-app review and public package verification passed. See the
[downloads](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.14) and
[qualification record](validation/2026-09-24-release-0.2.14/review.md).

## Live acquisition beside the analysis

Open the monitor from the top-bar Live indicator or dock it in the sidebar.
Acquisition continues while another stage or application is active. The monitor
has its own latest-scan, recent-scans and average choices; incoming data does not
change the spectrum selected for analysis.

Preview a representative completed file, choose its named signals, then select
individual scans, running averages or batches of a user-selected size. Selecting
transmission, fluorescence and reference produces three separate running averages
or three outputs per batch. Pause/resume reconciles new arrivals without counting
accepted revisions twice. Rewritten sources replace their averaging contribution
and retain their original batch position.

Choose a named processing reference independently of the current spectrum.
Preview captures its settings, and Start freezes them for the session; automatic
values still resolve for each incoming spectrum. Averages use equal weights over
common energy coverage and assume repeated measurements of the same state. They
do not infer alignment or a signal-to-noise stopping threshold. File completion
is inferred from stable observations and successful parsing. See the
[Live guide](live-acquisition.md) for recovery and operating limits.

## EXAFS curves and parameter trends

Select a reviewed single-spectrum EXAFS model for Live outputs, or choose
**Series → Add trend… → EXAFS fit…** to fit the frozen series membership.
Each fit uses the recorded model and starting values. The Series **Fit** view
shows data and model curves together with parameter trends. XANES peak fitting
remains a separately named operation.

Plot model variables, path distances or expressions such as `reff + dr_1`.
For a single-scattering path, this expression gives the absorber–scatterer
distance when `dr_1` is its fitted distance correction. Error bars show plus or
minus one local standard error, propagated using the retained fit covariance
and parameter correlations. They exclude model and calibration uncertainties.
Failed or unconverged fits leave gaps; missing covariance does not become zero
uncertainty. Connecting lines guide the eye rather than fitting a trend model.
The [Series fitting guide](series-fitting.md) explains the calculation and
assumptions. Saved projects retain full fit artifacts and custom expressions.

## Plot ranges

Right-click a plot and choose **Axis range…**. Each X or Y endpoint accepts a
number or **Auto**. A Y minimum of zero can remain fixed while the maximum follows
incoming data. Live and Series fit views retain ranges across frame updates, and
the main Live view shares limits with its monitor. Different parameters retain
separate ranges. These display preferences last for the application session;
they do not change fitting windows or source arrays. See
[plot axis ranges](plot-axis-ranges.md).

## Assistant apps and file import

**Access → Connected apps and files** enables integrations already installed and
connected in the user's Codex account. It starts off each session. Supported
one-time confirmations appear in the transcript, while login flows remain in
Codex. This option does not grant Workspace commands or change the analysis mode.

With **Edit analysis** enabled, the Assistant can queue explicitly approved local
spectrum files through the ordinary import and mapping review. Retained source
copies survive Assistant cleanup. Cloud files must first be locally accessible;
a sharing link alone is not a spectrum file. Automatic model selection prefers
GPT-6 Sol when the connected catalog offers it and respects explicit choices.
See the [Assistant guide](experimental-assistant.md).

## Compatibility and platforms

Project format remains 1. The 0.2.14 writer adds linked and embedded compatibility
fixtures; historical fixture bytes and data attribution remain intact. Portable
project resaves deduplicate identical source locators after verifying content,
and reject conflicting revisions without replacing the existing project.

Platform policy is unchanged: macOS desktop and Python distributions require
Apple Silicon. Windows and Linux desktop packages remain previews. The release
qualifies three Python ABI3 wheels on CPython 3.10–3.14 and keeps the Rust, Python,
npm/WebAssembly and desktop versions coordinated.
