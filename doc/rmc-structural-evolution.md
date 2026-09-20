# Adaptive memory and structural evolution in desktop RMC

These source-checkout features are **unreleased**. They extend the
[0.2.11 desktop workflow](rmc-desktop-workflow.md) without changing the exact
scattering model. Historical checkpoints keep their scientific settings.

## Cache memory and responsive preparation

**Fit settings → Cache memory (MiB)** defaults to **Auto**. Auto reads available
physical memory before preparation and reassesses it every two seconds. Let
`A` be available memory, `C` the retained scattering-cache payload, and `T` total
physical memory, all in bytes. The requested budget is

`min((min(A + C, T) − min(T/8, 1 GiB))₊ / 4, T/4, 8 GiB)`.

Here `(x)₊ = max(x, 0)`. The result is rounded down to whole MiB (1,048,576 bytes).
Adding the existing cache avoids interpreting its own allocation as pressure
from another application. The reserve and fractions are empirical rexafs resource
policies, not scientific parameters. A budget change of at least the smaller of
25% and 64 MiB is applied, with a 1 MiB minimum change; transitions to zero always
apply. The calculator enforces changes at an absorber-batch boundary. It allocates
snapshots as needed, rather than reserving the entire budget immediately.

Linux uses `MemAvailable`, Windows uses `ullAvailPhys`, and macOS estimates
reclaimable memory from free plus inactive pages. macOS free pages already include
speculative pages, so those are not added again. These counters are estimates,
not guarantees that an allocation will succeed. If a native query fails, Auto
uses 256 MiB and shows that fallback. See the authoritative
[Linux counters](https://docs.kernel.org/filesystems/proc.html),
[Windows counters](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-memorystatusex),
and [Apple counters](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/vm_statistics.h).

Enter a fixed MiB limit to override Auto, or clear it to restore Auto. The choice
applies to a new run or a cold checkpoint resume; a live paused worker keeps its
current policy. The limit bounds retained scattering snapshots only. Electronic
tables, path catalogues, temporary worker results, structural history and the rest
of the application use additional memory. Reducing the limit cannot instantly
release an in-progress batch or those other allocations.

The run bar reports path enumeration, electronic preparation and scattering before
the first completed move. Stop is checked during catalogue enumeration as well as
scattering. First-use **cold misses** are normal. **Repeated misses** mean that a
previously calculated context/grid no longer has a usable snapshot. Run details
shows hits, cold/repeated misses, evictions, oversized snapshots and estimated
payload for one snapshot per absorber. A capacity warning appears when that
estimate exceeds the limit. Recent repeated misses also produce a warning; it
clears after 30 seconds without another miss or after a budget increase. A
population search can benefit from additional snapshots beyond this minimum.

A cold resume preserves its last saved result before rebuilding the calculator.
Stopping preparation leaves that checkpoint available and reports an intentional
stop. A new run stopped before its initial evaluation has no computed state to
save.

Runtime resizing changes retention, not paths, precision or the accepted-state
calculation. The core constructor still defaults to 256 MiB and retains its
historical identity hashing. Rust callers can obtain `calculator.monitor()` and
call `set_cache_bytes(bytes)` without changing that identity. Constructor settings
must still match a saved checkpoint. `snapshot()` supplies thread-safe telemetry;
requesting a monitor activates progress instrumentation. See
[cache implementation](../crates/rexafs/src/xafs/rmc/accelerated.rs),
[monitor API](../crates/rexafs/src/xafs/rmc/prepared_monitor.rs), and
[desktop memory policy](../crates/rexafs-gui/src/rmc_fitting/memory.rs).

### Catalogue path capacity

**Fit settings → Catalogue path limit** controls the total number of retained
geometric paths across all absorber contexts. This is separate from **Cache
memory**, which stores calculated spectra for those paths. Increasing cache
memory does not raise the catalogue guard. The core API still defaults to one
million paths; earlier desktop builds used that fixed limit even for large cells.

For new desktop runs, **Auto** resolves the guard once from available memory.
It takes the cache-budget formula above with `C = 0`, caps that byte allowance
at 2 GiB, divides by an empirical allowance of 512 bytes per path, and rounds
down to an integer of at least one. This reserves a separate quarter of the
available pool for path identities and atom-to-path indices. The allowance is
a rexafs capacity heuristic, not a measured bound on resident memory; electronic
tables, temporary work and allocator overhead still need headroom. If the memory
query fails, the guard remains one million paths.

Enter an explicit count (1–100,000,000) to override Auto, or clear the field to
restore it. The resolved count is saved with the request and does not change on
resume; older requests preserve their historical guard and calculator identity.
Changing this field applies to a **new run**. A larger guard permits more paths
but allocates none immediately, changes no scattering approximation and never
samples absorbing sites. Per-absorber path and search-work guards still apply.

When preparation exceeds the guard, the reported count is a **lower bound found
so far**, not the final size of the unfinished catalogue. The desktop wraps the
message and offers **Path limit…** to return to Fit settings. Reduce the path
radius, maximum path legs or explicitly selected absorbers, or choose an adequate
path limit and start a new run. Changing radius, order or absorbing sites changes
the modeled calculation; increasing capacity alone does not. Implementation:
[request capture](../crates/rexafs-gui/src/rmc_fitting.rs),
[capacity policy](../crates/rexafs-gui/src/rmc_fitting/memory.rs), and
[enumeration guard](../crates/rexafs/src/xafs/rmc/accelerated.rs).

## Structural evolution

Choose **Results → Structural evolution** and an element pair, for example Cu–O
or Cu–Cu. Centers are the explicitly selected absorbing atoms, equally weighted;
neighbor species include every element present in the configuration. Initial,
current and best curves use the same bins. The distance heatmap places each
sample at its actual completed attempt or generation. Blank gaps carry no sample;
there is no interpolation across missing updates. Coordination, mean distance and
variance are plotted at the same actual sample coordinates.

Before a new run, **Fit settings → Structural tracking** selects the radial
interval, bin count and sampling policy. Defaults are [0, 8) Å, 160 equal-width
bins, every 100 RMC attempts and at most 256 retained history samples. Initial,
final, pause and checkpoint boundaries can add extra samples. Evolutionary runs
sample each completed generation. One active and one queued calculation bound the
background worker; if it falls behind, intermediate frames are skipped and
counted. Oldest history entries are removed at capacity, while the three overlays
are kept separately. Settings remain fixed for the run, including on resume;
changing bins does not reinterpret previously collected data. Historical
checkpoints without structural history remain usable and do not invent past
samples. Start a new run to enable tracking for those inputs.

The scientific distinction between a histogram and g(r) is explicit. For bin i
with lower/upper radii `r_i` and `r_(i+1)` in Å, let `H_AB,i` be the number of B
neighbors per selected A center in that bin. A periodic cell uses

`g_AB,i = H_AB,i / (rho_B * V_i)`,

where `rho_B = N_B / V_cell` is the number of B atoms per Å³ in the entire
explicit cell and `V_i = (4π/3) * (r_(i+1)³ − r_i³)` is the spherical shell volume
in Å³. The result is dimensionless. This is center, shell-volume and species-density
normalization as described by
[GROMACS](https://manual.gromacs.org/current/onlinehelp/gmx-rdf.html), with rexafs
using the full cell density rather than a local-density estimate. For A = B, the
central self image is excluded from counts; density still uses all N_B atoms,
without a finite-N correction. Other periodic self images are included. A finite
cluster has no assumed bulk density and is labeled **Neighbors per absorber per
bin**, not g(r).

Coordination is the sum of raw counts per center in the declared interval. Mean
distance is the arithmetic mean of all included neighbor distances in Å, and
variance is their population variance in Å². Empty intervals have zero
coordination and missing mean/variance, not zero-distance observations. These
moments cover the whole interval: choose a shell-specific interval to interpret
them as first-shell quantities. The code does not infer a shell boundary.

Periodic images are enumerated explicitly, including for skew cells. A warning
appears above half the smallest cell-plane spacing: those radii include repeated
cell correlations and do not supply independent bulk information. Finite-cluster
boundaries likewise reduce neighbor counts; no surface correction is invented.
Neither RMC attempts nor evolutionary generations are physical time, and an
optimization history is not an equilibrium ensemble or an uncertainty estimate.
Implementation and API definitions are in
[`radial_distribution`](../crates/rexafs/src/xafs/rmc/analysis.rs) and the
[background sampler](../crates/rexafs-gui/src/rmc_fitting/structural.rs).

Checkpoint/project saves retain settings, distributions and sampling gaps.
**Export result** adds `structural-evolution.json`, `structural-curves.csv` and
`structural-history.csv`; the CSVs include actual steps, atomic numbers, radial
bounds and units. A finite-cluster g(r) column is empty. These files contain
histograms and moments, not coordinates for every historical sample. Full current
and best coordinates remain in the numerical checkpoint.

## Genetic and hybrid search

**Fit settings → Search method** selects RMC, Genetic / EA, or Hybrid EA–RMC.
The evolutionary algorithm (EA) uses the existing Rust engine: tournament
selection, atom-wise crossover, mutation and elite survivors. Recommended starting
settings are 12 individuals, 50 generations and two elites. Pure genetic search
uses no local Metropolis moves; hybrid starts with one local attempt per nonelite
child. Population, generation budget, elites and local attempts are editable.
The desktop permits up to population size plus one retained snapshots per absorber
(capped at 1,024), subject to the shared byte limit.
Larger populations and local budgets increase scattering work; they do not solve
cache pressure automatically. See the
[engine description](rmc.md#evolutionary-search) and
[implementation](../crates/rexafs/src/xafs/rmc/evolution.rs) for mutation, diversity,
hypermutation and algorithmic scope. These are rexafs choices, not an EVAX replica.

Both evolutionary modes require fixed ΔE₀. Selecting them while energy refinement
is enabled produces an explicit validation error; calibration values are not
silently changed. Pause completes the current generation; Stop can interrupt a
calculation and retain the last committed population. Generations commit
transactionally. Preparation initializes the whole population, which can be
slower than ordinary RMC initialization even with a sufficient cache.

Progress reports generations and local attempts separately. The convergence
plot compares mean population objective with its best individual; it does not
apply the single-chain RMC plateau diagnostic. Structural Current and Best both
represent the best member of the current elitist population. Individuals are
never averaged into a fictitious structure. The Initial overlay remains the
original input before population mutation. Full population and random state are
saved, and continuation can extend the generation budget. Deterministic local
refinement remains a separate result and leaves the evolutionary checkpoint intact.
