"""Typed Rust spectrum processing. Energy: eV; k/q: inverse angstroms; R: angstroms."""

from typing import Literal, TypeAlias

import numpy as np
from numpy.typing import ArrayLike, NDArray

from . import io as io

__version__: str
"""Version of the installed Python package, for example "0.2.4". Include this value when reporting results or requesting help; a source build can contain changes beyond the published package with the same version."""

FFTGrid: TypeAlias = Literal["Input", "Larch"]
"""Sampling conventions for the forward transform: "Input" or "Larch".

"Input" preserves the background k grid. "Larch" linearly resamples onto a
zero-origin grid and extends the window domain to cover its upper taper.
Neither changes Spectrum.k() or Spectrum.chi(); use kwin_k() with kwin().
See [FFT grid compatibility](https://rexafs.com/docs/science/fourier-compatibility/)."""
FTWindow: TypeAlias = Literal[
    "Hanning", "Parzen", "Welch", "Gaussian", "Sine", "KaiserBessel", "FHanning"
]
"""Names of the supported real Fourier windows.

Hanning and FHanning use cosine tapers; Parzen is linear, Welch is parabolic,
Gaussian is bell-shaped, Sine uses a sine profile, and KaiserBessel uses a
modified Bessel function. Windows reduce truncation ringing but broaden peaks.
Taper parameters are shape-dependent: KaiserBessel also uses dk/dr to control
its shape, so equal parameter values do not make the windows equivalent.
Use the default for each processing stage as a starting point and inspect
the resulting window. See [Larch's window reference](https://xraypy.github.io/xraylarch/xafs_fourier.html#ftwindow)."""
AUTOBKSolver: TypeAlias = Literal["TrustRegionDogLeg", "LegacyLm", "LinearDirect"]
"""Names of the AUTOBK spline solvers.

"LinearDirect" solves a linear least-squares problem and is required by the
recommended "FixedPenalty" objective. "TrustRegionDogLeg" and "LegacyLm"
are iterative alternatives for the legacy "Fixed" or "TwoPass" objectives.
TrustRegionDogLeg requires the trust-region Rust feature, included in Python
packages. Changing the solver does not select a compatible objective for you."""
AUTOBKClampScalePolicy: TypeAlias = Literal["FixedPenalty", "Fixed", "TwoPass"]
"""Names of the AUTOBK endpoint-penalty models.

"FixedPenalty" is recommended: a fixed mean-square penalty discourages large
endpoint oscillations and uses LinearDirect. "Fixed" and "TwoPass" retain
legacy residual-dependent clamp scaling; TwoPass updates that scale after
an initial direct solve. The models solve different objectives and need not
produce identical backgrounds. See the [AUTOBK objective](https://rexafs.com/docs/science/autobk/)."""

class PrePostEdge:
    """Configure the baselines and edge step used to normalize absorption.

    A line is fitted before the absorption edge and a polynomial after it.
    The normalized signal is (mu - pre_edge) / edge_step; mu and both baselines
    have the same absorption units, so the result is dimensionless. The edge
    step is the fitted baseline difference near E0 unless you override it.
    Fit limits are energy offsets from E0 in eV, not absolute energies.

    All fields default to None and are resolved from the spectrum when processing.
    These automatic choices match Rust PrePostEdge::new(); Rust's Default trait
    uses fixed ranges instead. Start with automatic settings and inspect the
    baselines before choosing narrower ranges or a higher polynomial degree.
    Settings are copied into a method and spectrum; edit and assign them again
    to apply a later change.

    Example: p = PrePostEdge(); p.pre_edge_end = -30.0. Assign p to the
    spectrum's normalization stage, then call normalize() or a later stage.

    For the measurement and normalization convention, see
    [Newville, Fundamentals of XAFS, sections 4 and 5](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf).
    Automatic range selection and the numerical safeguards are rexafs choices;
    see [processing theory](https://rexafs.com/docs/science/processing/)."""
    def __init__(
        self,
        *,
        pre_edge_start: float | None = None,
        pre_edge_end: float | None = None,
        norm_start: float | None = None,
        norm_end: float | None = None,
        norm_polyorder: int | None = None,
        n_victoreen: int | None = None,
        e0: float | None = None,
        edge_step: float | None = None,
    ) -> None:
        """Create automatic pre/post-edge settings, with every field initially None.

        Set only the fields your data require, then assign the settings to the
        spectrum's normalization stage. Fit ranges and degrees are
        resolved when normalization runs; creating settings does not process data."""
    pre_edge_start: float | None
    """Start of the pre-edge fit, as an offset from E0 in eV.

    Default None estimates a lower bound near the second energy sample, rounded
    on a 2 or 5 eV grid and clipped to the measured range. Usually this offset
    is negative. Choose a range below the edge rise; reversing the two pre-edge
    bounds causes the implementation to swap them."""
    pre_edge_end: float | None
    """End of the pre-edge fit, as an offset from E0 in eV.

    Default None uses 5 * round(pre_edge_start / 15), approximately one third
    of the lower offset. A more negative value excludes more of the edge rise
    but leaves fewer baseline samples. Keep at least two usable pre-edge points."""
    norm_start: float | None
    """Start of the post-edge polynomial fit, as an offset from E0 in eV.

    Default None uses 5 * round(norm_end / 15), capped at 25 eV, then ensures
    at least a 10 eV separation from norm_end. Select a region beyond the edge
    rise; very short scans can make this automatic choice unsuitable."""
    norm_end: float | None
    """End of the post-edge polynomial fit, as an offset from E0 in eV.

    Default None rounds the available upper energy offset to a 5 eV grid and
    clips it to the data limit. Reducing this bound excludes high-energy data
    from the normalization fit; it does not trim the spectrum itself."""
    norm_polyorder: int | None
    """Degree of the polynomial fitted to the pre-edge-subtracted post-edge data.

    Default None chooses degree 0 for a fit span below 50 eV, 1 below 350 eV,
    and 2 otherwise. Explicit degrees are clamped to 0 through 5. Higher degrees
    can follow baseline curvature but also fit noise or oscillations; begin
    with the automatic choice. The fit needs more samples than its degree."""
    n_victoreen: int | None
    """Energy exponent used to fit the pre-edge baseline. Default None resolves to 0.

    For exponent n, the code fits mu(E) * E**n with a line, then divides that
    line by E**n to recover the baseline in mu units; E is in eV. With n=0
    this is an ordinary straight-line baseline. A nonzero exponent changes
    its curvature; it is a baseline model, not a correction for self-absorption."""
    e0: float | None
    """Absorption edge energy E0 in eV. Default: None for automatic detection.

    E0 defines the origin of the fit-range offsets and the conversion from
    energy to photoelectron k. Assigning these settings can replace a spectrum's
    previous E0. Spectrum processing rejects an explicit E0 that is non-finite
    or not strictly inside the measured energy range with ValueError; it does
    not silently replace such an invalid override with an automatic value. The
    lower-level Rust baseline fitter has its own redetection safeguards near the
    end of the scan, so inspect the resolved Spectrum.e0(). An automatic
    derivative estimate is not an energy calibration."""
    edge_step: float | None
    """Override the absorption edge step, in the same units as mu.

    Default None estimates post_edge - pre_edge at the sample nearest E0.
    Normalization divides by this value, so changing it rescales norm and chi.
    Use a finite positive value if a separately determined step is available.
    The implementation floors finite steps below 1e-12 to 1e-12; that safeguard
    does not make a zero or negative experimental edge step meaningful."""

class AUTOBK:
    """Fit a smooth atomic background and extract the EXAFS oscillation chi(k).

    AUTOBK fits a cubic spline in k to suppress low-R Fourier content of the
    background-subtracted signal. Subtracting that background and dividing
    by the normalization edge step gives dimensionless chi. rbkg controls the
    low-R region treated as background; a value that reaches the first
    structural shell can remove the signal you want to measure.

    Recommended starting values are rbkg=1.0 angstrom, kstep=0.05 inverse
    angstroms, kweight=1, window="Hanning", solver="LinearDirect", and
    clamp_scale_policy="FixedPenalty" with clamp_lambda=0.001. The fixed
    penalty discourages large endpoint oscillations; its strength is a
    rexafs convention, not a universal statistical regularization parameter.
    None resolves optional fields when processing, as described per field.

    Example: p = AUTOBK(); p.rbkg = 1.2. Assign p to the spectrum's background
    stage, then call calc_background() or a later stage.
    Settings are copied; assigning them again is required after later edits.
    Invalid ranges, an incompatible solver, or a failed solve raise RuntimeError
    during processing. A solved background still needs scientific inspection.

    Original AUTOBK method: [Newville et al. (1993)](https://doi.org/10.1103/PhysRevB.47.14126).
    The fixed endpoint penalty and linear solution are rexafs-specific choices;
    see the [implemented AUTOBK objective](https://rexafs.com/docs/science/autobk/)."""
    def __init__(
        self,
        *,
        ek0: float | None = None,
        rbkg: float | None = 1.0,
        nknots: int | None = None,
        kmin: float | None = 0.0,
        kmax: float | None = None,
        kstep: float | None = 0.05,
        nclamp: int | None = 3,
        clamp_lo: int | None = 0,
        clamp_hi: int | None = 1,
        clamp_lambda: float | None = 0.001,
        nfft: int | None = 2048,
        kweight: int | None = 1,
        dk: float | None = 0.1,
        linear_regularization: float | None = 0.0001,
        linear_condition_limit: float | None = 100000000.0,
        linear_residual_ratio_limit: float | None = 1.05,
        linear_fallback_to_lm: bool | None = True,
        linear_workspace_cache: bool | None = True,
        window: FTWindow | None = "Hanning",
        solver: AUTOBKSolver | None = "LinearDirect",
        linear_fallback_solver: AUTOBKSolver | None = "TrustRegionDogLeg",
        clamp_scale_policy: AUTOBKClampScalePolicy | None = "FixedPenalty",
    ) -> None:
        """Create AUTOBK settings with the recommended Rust defaults.

        Begin with rbkg=1.0 angstrom and the LinearDirect/FixedPenalty pair.
        Edit only the fields your data require, then assign the settings to the
        spectrum's background stage. Construction does not fit a spectrum;
        numeric range and solver compatibility checks occur during processing."""
    ek0: float | None
    """Edge energy used to convert energy to k, in eV.

    Default None uses the normalization E0. Above the edge, k is approximately
    sqrt((E - E0) / 3.81), with E in eV and k in inverse
    angstroms; the conversion constant has units eV * angstrom**2.
    Prefer Spectrum.set_e0() when changing the edge for all stages
    together; an independent background edge can differ from normalization."""
    rbkg: float | None
    """Positive background cutoff in angstroms. Default: 1.0; None resolves to 1.0.

    AUTOBK suppresses low-R Fourier residuals up to a cutoff derived from this
    value, the k range, and the discrete R grid; it is not an exact continuous
    boundary. Increasing rbkg allows a more flexible spline and can remove
    real short-distance structure. Start near 1.0 and keep it below the first
    physical shell of interest."""
    nknots: int | None
    """Requested number of spline coefficients/anchor points.

    Default None uses 1 + floor(2 * rbkg * (kmax - kmin) / pi), clamped to
    5 through 128; explicit values are clamped to the same range. rbkg is in
    angstroms and k limits in inverse angstroms, so the count is dimensionless.
    More coefficients increase background flexibility. Leave this automatic
    unless testing a justified spline model."""
    kmin: float | None
    """Lower k bound of the background fit and Fourier window, in inverse
    angstroms.

    Default: 0.0. None also resolves to 0.0. Raising it excludes low-k data from
    the Fourier objective and changes the spline geometry. It must be below the
    effective kmax. The returned k() array still starts at zero."""
    kmax: float | None
    """Upper k bound of the background fit, in inverse angstroms.

    Default None uses the available data limit; an explicit larger value is
    clipped to that limit. Lower it to exclude a noisy high-k tail. This also
    changes the spline count and the extent of the returned k()/chi() arrays;
    it is independent of the later XrayFFTF.kmax setting."""
    kstep: float | None
    """Spacing of the uniform output k grid, in inverse angstroms.

    Default: 0.05. None also resolves to 0.05. Values must be finite and
    positive. The grid begins at zero and is used by Spectrum.k() and chi().
    Smaller steps produce more interpolated samples without adding measured
    information. This usually supplies the automatic spacing of the later
    forward transform."""
    nclamp: int | None
    """Number of endpoint samples included at each enabled end of the clamp.

    Default: 3. None also resolves to 3; 0 disables endpoint penalties.
    FixedPenalty uses the first and last nclamp samples, limited to the
    available length, and requires a nonnegative value. Increasing the count
    constrains a wider endpoint region. clamp_lo/clamp_hi select which ends
    contribute."""
    clamp_lo: int | None
    """Relative weight of the low-k endpoint penalty. Default: 0. None also
    resolves to 0.

    Zero disables this end. For FixedPenalty, the absolute integer weight
    multiplies the endpoint residual before squaring; doubling it gives four
    times the squared contribution for a fixed residual. Start at zero so the
    near-edge oscillation is not forced toward zero."""
    clamp_hi: int | None
    """Relative weight of the high-k endpoint penalty. Default: 1. None also
    resolves to 1.

    Zero disables this end. For FixedPenalty, the absolute integer weight
    multiplies endpoint residuals before squaring. Larger values discourage
    large high-k endpoint oscillations more strongly but may bias a real
    oscillation; clamp_lambda controls the overall penalty strength."""
    clamp_lambda: float | None
    """Strength of the FixedPenalty mean-square endpoint term. Default: 0.001.

    None resolves to 0.001; 0 disables both endpoint penalties regardless of
    their weights. Values must be finite and nonnegative. Increasing this
    value favors smaller endpoint oscillations over the low-R objective.
    Its numerical meaning depends on the implemented Fourier scaling and
    weights; it is not an uncertainty estimate. Unused by Fixed and TwoPass."""
    nfft: int | None
    """Number of samples in the FFT used inside background removal.

    Default: 2048. None also resolves to 2048. Use a positive length large
    enough to contain the output k grid: the underlying FFT otherwise keeps only
    its first nfft samples. A larger length makes the R grid finer through zero
    padding; it does not improve experimental resolution. This setting is
    independent of the later XrayFFTF.nfft."""
    kweight: int | None
    """Integer exponent of k in the background Fourier objective. Default: 1.

    None resolves to 1. Larger nonnegative weights emphasize higher-k
    oscillations and their noise while the spline is fitted. Use nonnegative
    values on the zero-origin grid; negative powers are singular at k=0.
    This weight does not remain in the returned chi() and is independent
    of the exponent used by the later forward transform."""
    dk: float | None
    """Window taper parameter for the background objective. Default: 0.1.

    None resolves to 0.1. For the default Hanning window this sets the taper
    width at each k bound, in inverse angstroms. A broader taper softens
    truncation but reduces the strongly weighted range. Other window families
    interpret the parameter differently; KaiserBessel also uses it as a
    shape parameter. Inspect the window when changing families."""
    linear_regularization: float | None
    """Ridge strength for legacy Fixed/TwoPass direct solves. Default: 0.0001.

    None resolves to 0.0001. It adds a scaled diagonal regularization to the
    legacy normal equations, trading closeness to the unregularized solution
    for stability. FixedPenalty does not use this parameter; changing it
    does not regularize the recommended column-scaled SVD solve."""
    linear_condition_limit: float | None
    """Largest accepted condition measure for a direct solve. Default: 1e8.

    None resolves to 1e8. FixedPenalty requires a finite value at least 1
    and checks the largest/smallest singular-value ratio of its column-scaled
    design matrix. Legacy solves use a different condition proxy. Lowering
    the limit rejects more unstable systems; raising it can accept sensitive
    solutions. FixedPenalty failures raise RuntimeError without a fallback."""
    linear_residual_ratio_limit: float | None
    """Maximum solved/base residual-norm ratio for legacy direct solves.

    Default: 1.05. None also resolves to 1.05; legacy processing clamps it to at
    least 1. Exceeding the limit rejects the direct solution and may trigger the
    configured fallback. FixedPenalty does not use this legacy acceptance test;
    its rank, condition and stationarity checks are separate."""
    linear_fallback_to_lm: bool | None
    """Allow a rejected legacy direct solve to use linear_fallback_solver.

    Default: True. None also resolves to True. Despite its historical name, the
    selected fallback need not be Levenberg-Marquardt. False makes a rejected
    legacy direct solve raise an error. FixedPenalty never falls back: it
    returns a failed solve as RuntimeError regardless of this setting."""
    linear_workspace_cache: bool | None
    """Reuse compatible background-solver geometry and factorization. Default: True.

    None resolves to True. FixedPenalty caches spline/FFT design matrices,
    column scaling and singular-value decomposition factors for matching
    geometry; each spectrum still supplies its own data and receives a new
    solution. False disables this reuse, mainly for comparisons or profiling;
    the cache does not reuse a previous spectrum's chi()."""
    window: FTWindow | None
    """Fourier window used inside the background objective. Default: "Hanning".

    None selects Hanning. A window reduces artifacts from abrupt k truncation;
    its shape changes the objective and can change the extracted background.
    See FTWindow for accepted names and shape-dependent taper behavior.
    An unsupported name raises ValueError when assigned."""
    solver: AUTOBKSolver | None
    """Solver used for the spline coefficients. Recommended default: "LinearDirect".

    None resolves to LinearDirect, which is required by FixedPenalty.
    TrustRegionDogLeg and LegacyLm are iterative solvers for legacy objectives;
    select Fixed or TwoPass explicitly to use them. TrustRegionDogLeg requires
    the trust-region Rust feature, included in Python packages. Unknown names
    raise ValueError; incompatible solver/objective pairs fail during processing."""
    linear_fallback_solver: AUTOBKSolver | None
    """Solver used after a rejected legacy LinearDirect solve.

    The Python constructor default is "TrustRegionDogLeg". Explicit None
    normally resolves to "LegacyLm" when fallback is enabled; it is therefore
    different from leaving the constructor argument unchanged. LinearDirect
    cannot be its own fallback. FixedPenalty ignores this setting and never
    falls back. Unknown names raise ValueError when assigned."""
    clamp_scale_policy: AUTOBKClampScalePolicy | None
    """Endpoint model for background removal. Recommended default: "FixedPenalty".

    None resolves to FixedPenalty, which requires LinearDirect and uses
    clamp_lambda as a fixed mean-square penalty strength. Fixed and TwoPass
    retain older residual-dependent clamp models; their results need not
    match the recommended objective. See AUTOBKClampScalePolicy for the
    distinction. Unknown names raise ValueError when assigned."""

class XrayFFTF:
    """Configure the weighted Fourier transform from photoelectron k to distance R.

    The code forms g = chi * k**w * W, with integer w=kweight and real window
    W, then computes chi_R = (kstep / sqrt(pi)) * rfft(g, n=nfft) using an
    unnormalized, negative-exponent FFT. There is no additional 1/nfft factor,
    window-area normalization or phase rotation. On a zero-origin uniform
    grid, R_m = pi * m / (nfft * kstep), where m is the frequency-bin index.
    For dimensionless chi, chi_R has units angstrom**(-(w + 1)); R is in
    angstroms. Zero padding refines the R sampling but adds no measured data.

    Recommended defaults are kmin=2, kmax=15 inverse angstroms, kweight=2,
    window="KaiserBessel", dk=1, nfft=2048 and grid="Input". Automatic kstep
    uses the background grid, usually 0.05 inverse angstroms. Settings are
    copied into a spectrum, so reassign them after editing.

    Example: p = XrayFFTF(); p.kmax = 12.0; spectrum.set_fft(p).fft().
    Use spectrum.kwin_k() with kwin() to inspect the window. Scattering phase
    shifts mean that an uncorrected Fourier peak is not directly a bond length.

    Implementation convention: [NumPy's DFT definition](https://numpy.org/doc/stable/reference/routines.fft.html#implementation-details).
    Physical interpretation: [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).
    See [processing theory](https://rexafs.com/docs/science/processing/) for the
    equation and links to the implementing Rust functions."""
    def __init__(
        self,
        *,
        grid: FFTGrid = "Input",
        rmax_out: float | None = 10.0,
        dk: float | None = 1.0,
        dk2: float | None = None,
        kmin: float | None = 2.0,
        kmax: float | None = 15.0,
        kweight: float | None = 2.0,
        nfft: int | None = 2048,
        kstep: float | None = None,
        window: FTWindow | None = "KaiserBessel",
    ) -> None:
        """Create forward-transform settings with the recommended Rust defaults.

        Start with k=2 to 15 inverse angstroms, kweight=2, a KaiserBessel window
        and nfft=2048. Choose a useful k range for your measured data, then call
        spectrum.set_fft(parameters).fft(). Construction does not run a transform;
        numeric validation occurs when fft() processes the data."""
    grid: FFTGrid
    """Sampling/window convention. Default: "Input".

    Input keeps the existing background samples and does not resample them
    when kstep changes. Larch linearly interpolates onto a zero-origin grid
    with the requested kstep and extends the window domain through the upper
    taper. Both preserve Spectrum.k()/chi(); kwin_k() returns the matching
    window axis. Unknown names raise ValueError when assigned."""
    rmax_out: float | None
    """Largest R shown by r() and chir_real/imag/mag(), in angstroms.

    Default: 10.0. None also resolves to 10.0. It must be finite and
    nonnegative. This limits returned display arrays; the complete internal
    forward transform is retained for inverse filtering. Increasing it does not
    improve spatial resolution or apply a structural shell filter."""
    dk: float | None
    """Low-k window taper parameter. Default: 1.0. None also resolves to 1.0.

    For Hanning-like windows it describes a width in inverse angstroms.
    KaiserBessel also uses this numeric value to set the Bessel-function shape,
    so it is not a universally comparable taper width. Larger tapers generally
    soften truncation at the cost of a broader R response. Use kwin_k()/kwin()
    to inspect the actual window."""
    dk2: float | None
    """High-k window taper parameter. Default None uses dk.

    It controls the upper-end geometry in inverse angstroms for the
    width-based windows. Set it separately for an asymmetric taper.
    Window families interpret taper parameters differently; KaiserBessel's
    Bessel-function shape is controlled by dk, not an independent dk2 shape."""
    kmin: float | None
    """Lower Fourier-window limit, in inverse angstroms. Constructor default: 2.0.

    Explicit None uses the first background k sample, usually zero. Raising
    the limit suppresses low-k contributions but shortens the effective
    transform range. It must be finite, nonnegative and below kmax.
    The taper can extend below this nominal limit."""
    kmax: float | None
    """Upper Fourier-window limit, in inverse angstroms. Constructor default: 15.0.

    Explicit None uses the last background k sample. Lower it to exclude
    a noisy high-k tail; this changes the transform without recomputing chi.
    The taper can extend above the nominal bound. Input and Larch use
    different window domains when the taper reaches beyond measured data."""
    kweight: float | None
    """Exponent applied as k**w before the transform. Default: 2.0. None also
    resolves to 2.0.

    Finite nonnegative values are floored to an integer w. A higher weight
    emphasizes high-k oscillations and noise. It changes both amplitude and
    units: dimensionless chi gives chi(R) in angstrom**(-(w + 1)). The
    background chi() remains unweighted."""
    nfft: int | None
    """Total FFT length, including zero padding. Default: 2048. None also resolves
    to 2048.

    It must be at least 2. Choose a length that contains the prepared data:
    Input keeps only the first nfft samples if the data are longer; Larch
    requires its extended window grid to fit. Increasing nfft gives a finer R
    grid with spacing pi / (nfft * kstep), but does not add structural
    information or introduce an extra amplitude normalization."""
    kstep: float | None
    """k spacing used for FFT scaling and the R axis, in inverse angstroms.

    Default None uses the difference between the first two input k values;
    the default AUTOBK grid gives 0.05. An explicit value must be finite and
    positive. Input does not resample, so keep this equal to its actual grid
    spacing. Use grid="Larch" when requesting resampling at a different step.
    The forward amplitude multiplier is kstep / sqrt(pi)."""
    window: FTWindow | None
    """Forward Fourier-window shape. Constructor default: "KaiserBessel".

    Explicit None selects Hanning, which differs from leaving the default
    unchanged. The window reduces truncation ringing and broadens the R
    response; it is not normalized by its area. See FTWindow for accepted
    names. Unknown names raise ValueError when assigned."""

class XrayFFTR:
    """Configure a real inverse Fourier transform with an R-space window.

    The code weights positive-R bins by R**rweight and a real window, then
    reconstructs their conjugate negative-frequency partners to obtain a real
    signal on q. q has the same physical meaning as k, in inverse angstroms;
    the different name identifies a back-transformed, potentially filtered
    signal. The forward k weighting and window remain in that signal.
    This real inverse differs from Larch's complex, one-sided back-transform.

    Defaults are rmin=0, rmax=20 angstroms, rweight=0, a KaiserBessel window,
    dr=1 angstrom, qmax_out=10 inverse angstroms, nfft=2048, and automatic
    kstep. Choose an R interval appropriate to your shells for deliberate
    filtering. The uncorrected R axis includes scattering phase shifts.

    Example: p = XrayFFTR(); p.rmin = 1.0; p.rmax = 3.0;
    spectrum.set_ifft(p).ifft(). Settings are copied; reassign after edits.
    These Python settings and set_ifft() are additions after 0.2.4.

    See the [implemented inverse convention](https://rexafs.com/docs/science/processing/)
    for the scaling, and [Larch's Fourier guide](https://xraypy.github.io/xraylarch/xafs_fourier.html)
    for windowing concepts rather than an assertion of identical inverse output."""
    def __init__(
        self,
        *,
        qmax_out: float | None = 10.0,
        dr: float | None = 1.0,
        dr2: float | None = None,
        rmin: float | None = 0.0,
        rmax: float | None = 20.0,
        rweight: float | None = 0.0,
        nfft: int | None = 2048,
        kstep: float | None = None,
        window: FTWindow | None = "KaiserBessel",
    ) -> None:
        """Create inverse-transform settings with the recommended Rust defaults.

        Set rmin/rmax to select an R region and use rweight=0 unless extra R
        weighting is intended. Assign with spectrum.set_ifft(parameters).ifft().
        The returned signal retains forward weighting and windowing; construction
        alone does not filter a spectrum."""
    qmax_out: float | None
    """Largest returned q value, in inverse angstroms. Default: 10.0. None also
    resolves to 10.0.

    The q()/chiq() arrays stop at this bound or the available inverse samples,
    whichever comes first. It must be finite and nonnegative. This limits
    returned samples and does not change the R-space filter."""
    dr: float | None
    """Low-R window taper parameter. Default: 1.0. None also resolves to 1.0.

    For width-based windows the unit is angstroms. KaiserBessel also uses this
    numeric value as its Bessel-function shape parameter. A wider taper smooths
    the selected R boundary but mixes a broader range of distances into the
    filtered signal."""
    dr2: float | None
    """High-R window taper parameter. Default None uses dr.

    Use a separate value for asymmetric R-window geometry. The unit is
    angstroms for width-based windows; KaiserBessel's Bessel-function shape
    uses dr, so dr2 does not define an independent high-end shape."""
    rmin: float | None
    """Lower R-window limit, in angstroms. Constructor default: 0.0.

    Explicit None uses the first input R sample, which is zero. A larger
    value removes lower-R contributions from the back-transform. The bound
    must be finite, nonnegative and below rmax; the taper can extend below it.
    R peaks have not been corrected for scattering phase shifts."""
    rmax: float | None
    """Upper R-window limit, in angstroms. Constructor default: 20.0.

    Choose a shell range such as 1 to 3 angstroms only when appropriate for
    your data. Explicit None uses the last reported input R sample, which
    depends on forward rmax_out; an explicit bound instead acts on the full
    internal Fourier bins. The taper can extend above the nominal bound."""
    rweight: float | None
    """Exponent applied as R**v before the inverse transform. Default: 0.0.

    None resolves to 0.0; finite nonnegative values are floored to an integer
    v. Leave it at zero for ordinary R filtering. A positive value emphasizes
    higher-R contributions and changes the units of chiq(): with forward
    kweight w the units are angstrom**(v - w)."""
    nfft: int | None
    """Inverse transform length. Default: 2048. None also resolves to 2048;
    minimum: 2.

    Leave kstep automatic when changing this value. With fixed input R spacing,
    a larger nfft gives a finer q grid by zero padding Fourier bins; a smaller
    value discards bins beyond its representable range. Neither operation adds
    experimental information."""
    kstep: float | None
    """Output q spacing, in inverse angstroms.

    Default None uses pi / (nfft * delta_R), where delta_R is the difference
    between the first two R samples in angstroms. An explicit positive value
    must agree with that spacing or processing raises RuntimeError. Keep it
    automatic when changing nfft so the physical Fourier grid stays consistent."""
    window: FTWindow | None
    """R-space Fourier-window shape. Constructor default: "KaiserBessel".

    Explicit None selects Hanning, unlike leaving the default unchanged.
    The window selects and tapers R contributions before the real inverse;
    it cannot undo the weighting or information lost in the forward window.
    See FTWindow for choices. Unknown names raise ValueError when assigned."""

class NormalizationMethod:
    """Select the normalization algorithm and own a copy of its settings.

    Use NormalizationMethod.PrePostEdge(parameters) for configured pre/post-edge
    normalization or new_prepostedge() for automatic settings, then pass the
    result to Spectrum.set_normalization_method(). Creating a method does not
    process data. MBack is a named placeholder and is not implemented."""
    @staticmethod
    def PrePostEdge(parameters: PrePostEdge) -> NormalizationMethod:
        """Copy PrePostEdge parameters into a normalization method.

        Assign the result with spectrum.set_normalization_method(method), then
        call normalize() or a later stage. The original parameters remain
        independent: edits do not change an already created method or spectrum."""
    @staticmethod
    def new_prepostedge() -> NormalizationMethod:
        """Create a normalization method with automatic pre/post-edge settings.

        E0, fitting ranges, polynomial degree and edge step are inferred when
        processing runs. Equivalent to NormalizationMethod.PrePostEdge(PrePostEdge());
        assign the method to a spectrum to use it. No data are processed here."""
    @staticmethod
    def new_mback() -> NormalizationMethod:
        """Create the unimplemented MBack normalization placeholder.

        Selecting it preserves the requested algorithm, but normalize() and
        dependent stages raise ValueError rather than substitute another method.
        Use new_prepostedge() for the implemented normalization workflow."""

class BackgroundMethod:
    """Select a background algorithm and own a copy of its settings.

    Use BackgroundMethod.AUTOBK(parameters) to configure the spline background
    or new_autobk() for the recommended defaults, then pass the result to
    Spectrum.set_background_method(). Creating a method does not process
    data. ILPBkg is a named placeholder and is not implemented."""
    @staticmethod
    def AUTOBK(parameters: AUTOBK) -> BackgroundMethod:
        """Copy AUTOBK parameters into a background method.

        Assign the result with spectrum.set_background_method(method), then call
        calc_background() or a later stage. Edits to the original parameters do
        not change an already created method or spectrum."""
    @staticmethod
    def new_autobk() -> BackgroundMethod:
        """Create a background method using the recommended AUTOBK defaults.

        This selects rbkg=1.0 angstrom, LinearDirect and FixedPenalty with
        clamp_lambda=0.001. Assign it to a spectrum before processing; no
        background is fitted by this factory."""
    @staticmethod
    def new_ilpbkg() -> BackgroundMethod:
        """Create the unimplemented ILPBkg background placeholder.

        Selecting it preserves the requested algorithm, but calc_background()
        and dependent stages raise RuntimeError rather than substitute AUTOBK.
        Use new_autobk() for the implemented background workflow."""

class Spectrum:
    """Own a measured absorption spectrum and its calculated processing stages.

    Energy is in eV; absorption mu may be an absorption coefficient or a
    consistently scaled measurement such as transmission optical depth.
    Input arrays are copied to Rust-owned float64 storage. Returned NumPy
    arrays are independent copies, so editing them does not change this
    spectrum. Each result getter returns None until its stage has succeeded.

    Start with Spectrum(energy, mu).fft() to run automatic normalization,
    AUTOBK background removal and the k-to-R transform. Explicit processing
    calls recompute their stage and return this same object; missing
    prerequisites run automatically with the selected settings. Calculation
    methods release Python's global interpreter lock while Rust runs.

    Changing input data or settings clears dependent results. A failed stage
    does not substitute another algorithm. Invalid input and normalization
    errors raise ValueError; background and Fourier errors raise RuntimeError.
    See [processing theory](https://rexafs.com/docs/science/processing/) for
    equations, interpretation and limitations. Groups and structural fitting
    are not currently exposed by this Python Spectrum API."""
    def __init__(self, energy: ArrayLike, mu: ArrayLike) -> None:
        """Create a spectrum by copying energy and mu into float64 storage.

        energy contains X-ray energies in eV; mu contains the corresponding
        absorption values in consistent units. Lists, NumPy arrays and strided
        views are accepted. Inputs must be one-dimensional, finite, equal-length,
        have at least two samples, and have strictly increasing energy.
        Invalid shapes or data raise ValueError. No processing runs here;
        call fft() for the default pipeline or normalize() for just normalization."""
    @staticmethod
    def from_arrays(energy: ArrayLike, mu: ArrayLike) -> Spectrum:
        """Create a spectrum from energy in eV and corresponding absorption mu.

        Equivalent to Spectrum(energy, mu): it accepts array-like inputs and
        copies them to owned float64 storage. Inputs must be finite, one-dimensional,
        equal-length, with at least two samples and strictly increasing energy.
        Invalid data raise ValueError. Derived results are initially unavailable."""
    def set_spectrum(self, energy: ArrayLike, mu: ArrayLike) -> Spectrum:
        """Replace measured energy/mu, copy the inputs and clear E0 and derived results.

        Input units and validation match the constructor: energy in eV, finite
        matching one-dimensional arrays with strictly increasing energy and at
        least two samples. Invalid input raises ValueError before replacing data.
        Stage settings are retained, but old calculated edge values and arrays
        are discarded. Returns this spectrum; call a processing stage to recompute."""
    def set_e0(self, e0: float) -> Spectrum:
        """Set the absorption edge energy in eV and clear all dependent results.

        The value is propagated to configured normalization and AUTOBK settings,
        so subsequent energy-to-k conversion uses the new edge. Use a finite
        value strictly inside the measured energy range; validation occurs when
        processing runs, not at this setter. Returns this spectrum without
        performing normalization or recalibrating the input energy axis."""
    def set_normalization_method(
        self, method: PrePostEdge | NormalizationMethod | None = None
    ) -> Spectrum:
        """Copy the selected normalization method and clear normalization and later results.

        Source builds accept PrePostEdge settings directly; omitted/None restores
        automatic normalization. Published 0.2.4 uses
        NormalizationMethod.PrePostEdge(parameters) or new_prepostedge().
        An explicit E0 in those parameters becomes the spectrum E0; otherwise
        an existing spectrum E0 is retained. Edits to the original settings do
        not propagate: assign again to apply them. Returns this spectrum without
        processing data; call normalize() or a later stage to recompute."""
    def set_background_method(
        self, method: AUTOBK | BackgroundMethod | None = None
    ) -> Spectrum:
        """Copy the selected background method and clear background and Fourier results.

        Source builds accept AUTOBK settings directly; omitted/None restores
        default AUTOBK. Published 0.2.4 uses BackgroundMethod.AUTOBK(parameters)
        or BackgroundMethod.new_autobk() for recommended defaults. Existing
        normalization results are retained. Later edits to the original settings
        do not propagate: assign them again to apply changes. Returns this
        spectrum without fitting; call calc_background() or a later stage."""
    def set_ifft(self, parameters: XrayFFTR) -> Spectrum:
        """Copy inverse settings and clear q()/chiq(), preserving forward results.

        Use XrayFFTR to choose the R window and output q range. Editing the
        original settings later does not change this spectrum; assign again
        to apply changes. Returns this spectrum without filtering. Call ifft()
        to compute the new result. This binding is an addition after 0.2.4."""
    def set_fft(self, parameters: XrayFFTF) -> Spectrum:
        """Copy forward settings and clear Fourier and inverse results.

        Normalization and the background k()/chi() arrays are retained. Use
        XrayFFTF to choose k weights, the window and grid convention. Editing
        the original settings later has no effect until you assign them again.
        Returns this spectrum without transforming; call fft() to recompute."""
    def e0(self) -> float | None:
        """Return the detected or assigned edge energy E0 in eV, or None if unset.

        This getter does not run edge detection. Call find_e0(), normalize() or
        a later processing stage to resolve an automatic edge. E0 sets the
        energy origin for k conversion; it is not an independent calibration
        measurement or a fitted structural energy shift."""
    def find_e0(self) -> Spectrum:
        """Estimate the absorption edge from the energy derivative of mu.

        The detector seeks a strong rise with neighboring high-derivative
        samples and refines its search around the candidate edge. It returns
        this spectrum, stores E0 in eV and clears normalization and all later
        results. Inspect the result for noisy spectra or multiple edges;
        automatic detection is not energy calibration. Invalid data raise ValueError."""
    def normalize(self) -> Spectrum:
        """Fit pre/post-edge baselines and compute dimensionless normalized absorption.

        The implemented method computes norm = (mu - pre_edge) / edge_step
        and flat with its fitted post-edge trend removed. Missing E0 and
        automatic parameters are resolved from the data. Call norm(), flat(),
        pre_edge() and post_edge() to retrieve independent result arrays.
        This recomputes normalization, clears background/Fourier results and
        returns this spectrum. Invalid ranges, failed fits and MBack raise ValueError."""
    def calc_background(self) -> Spectrum:
        """Fit the selected smooth background and calculate dimensionless chi(k).

        Missing normalization runs first. The default AUTOBK method suppresses
        low-R Fourier content with rbkg=1.0 angstrom and the fixed endpoint
        penalty; k() is a zero-origin grid with default step 0.05 inverse angstroms.
        The returned chi() is unweighted. This recomputes the background, clears
        forward/inverse results and returns this spectrum. Background failures
        or ILPBkg raise RuntimeError; prerequisite normalization can raise ValueError."""
    def fft(self) -> Spectrum:
        """Compute the weighted k-to-R Fourier transform, running missing prerequisites.

        Defaults are k=2 to 15 inverse angstroms, kweight=2, a KaiserBessel
        window and nfft=2048. The amplitude multiplier is kstep / sqrt(pi)
        after an unnormalized negative-exponent FFT; there is no extra 1/nfft.
        For dimensionless chi the default output units are inverse cubic
        angstroms. XrayFFTF documents the full convention and sampling choices.

        Use r() with chir_real(), chir_imag() or chir_mag(), and kwin_k() with
        kwin(). This recomputes the forward transform, clears inverse results
        and returns this spectrum. Background and FFT failures raise RuntimeError;
        prerequisite normalization can raise ValueError."""
    def ifft(self) -> Spectrum:
        """Back-transform the stored Fourier signal to a real signal on q().

        Missing forward stages run first. The inverse applies an R window and
        reconstructs a real signal using conjugate negative-frequency bins.
        The forward k weighting and window remain: chiq() is generally not
        the original unweighted chi(k). With default forward kweight=2 and
        inverse rweight=0, chiq() has units inverse square angstroms.
        This recomputes the inverse and returns this spectrum. Invalid inverse
        grids or settings raise RuntimeError; prerequisite errors propagate."""
    def invalidate_derived(self) -> Spectrum:
        """Clear normalization, background and Fourier results, keeping stage settings.

        The measured inputs and spectrum E0 are retained, as are explicit
        settings needed for recomputation. Getters for cleared arrays return
        None until their stages run again. Ordinary setters already invalidate
        the affected results; use this method when you need a full recomputation.
        Returns this spectrum without running any calculations."""
    def k(self) -> NDArray[np.float64] | None:
        """Uniform background k axis in inverse angstroms, paired with chi().

        It begins at zero with AUTOBK.kstep spacing. It remains the background
        grid even when the forward transform uses grid="Larch".

        Returns an independent NumPy float64 array copy, or None before
        calc_background() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def chi(self) -> NDArray[np.float64] | None:
        """Unweighted, dimensionless EXAFS oscillation on k().

        chi is (mu - smooth background) / edge_step, resampled on the background
        k grid. Neither the AUTOBK objective weight nor the later forward FFT
        k weight is stored in this array.

        Returns an independent NumPy float64 array copy, or None before
        calc_background() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def norm(self) -> NDArray[np.float64] | None:
        """Dimensionless normalized absorption on the original energy grid.

        norm = (mu - pre_edge) / edge_step, with the baseline and edge step in
        the same units as mu. Use this for a normalized edge; flat() additionally
        removes the fitted post-edge trend.

        Returns an independent NumPy float64 array copy, or None before
        normalize() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def flat(self) -> NDArray[np.float64] | None:
        """Dimensionless normalized absorption with the fitted post-edge trend removed.

        Above the sample nearest E0, the fitted baseline difference divided
        by edge_step is subtracted and its value at E0 added back. Below that
        sample, flat equals norm. Values use the original energy grid; this
        flattening is separate from AUTOBK background subtraction.

        Returns an independent NumPy float64 array copy, or None before
        normalize() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def pre_edge(self) -> NDArray[np.float64] | None:
        """Fitted pre-edge baseline on the original energy grid, in mu units.

        The line fitted to mu * E**n_victoreen is divided by E**n_victoreen.
        It is extrapolated across the scan so normalization can subtract it.
        Inspect this baseline against the measured pre-edge region.

        Returns an independent NumPy float64 array copy, or None before
        normalize() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def post_edge(self) -> NDArray[np.float64] | None:
        """Fitted post-edge baseline on the original energy grid, in mu units.

        This is the fitted post-edge polynomial plus the pre-edge baseline.
        Its difference from pre_edge near E0 estimates the normalization edge
        step. It is not the AUTOBK smooth atomic background.

        Returns an independent NumPy float64 array copy, or None before
        normalize() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def r(self) -> NDArray[np.float64] | None:
        """Forward-transform R axis in angstroms, paired with chir_real/imag/mag().

        For FFT length N and k spacing delta_k, adjacent bins are separated
        by pi / (N * delta_k); rmax_out limits the returned range. Zero padding
        only refines sampling. Scattering phase shifts mean that an uncorrected
        R peak is not directly a bond length.

        Returns an independent NumPy float64 array copy, or None before
        fft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def kwin(self) -> NDArray[np.float64] | None:
        """Dimensionless forward Fourier-window values, paired with kwin_k().

        These are the window W before multiplying by chi and k**kweight.
        Use the matching kwin_k() axis: with grid="Larch" the window can extend
        beyond, and have a different length from, the background k() grid.

        Returns an independent NumPy float64 array copy, or None before
        fft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def kwin_k(self) -> NDArray[np.float64] | None:
        """k axis of the forward Fourier window, in inverse angstroms.

        Pair it with kwin(). For grid="Input" it matches the background grid;
        for grid="Larch" it uses the resampled, extended window domain. The
        background k()/chi() arrays themselves are unchanged by this choice.

        Returns an independent NumPy float64 array copy, or None before
        fft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def chir_mag(self) -> NDArray[np.float64] | None:
        """Magnitude of the complex forward transform, paired with r().

        It equals sqrt(chir_real()**2 + chir_imag()**2). For dimensionless chi
        and integer forward kweight w, units are angstrom**(-(w + 1)); the
        default w=2 gives inverse cubic angstroms. Magnitude discards phase
        information and is not a probability density.

        Returns an independent NumPy float64 array copy, or None before
        fft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def chir_real(self) -> NDArray[np.float64] | None:
        """Real component of the complex forward transform, paired with r().

        The convention uses a negative-exponent FFT followed by kstep/sqrt(pi),
        without additional FFT-length normalization. For dimensionless chi and
        integer kweight w, units are angstrom**(-(w + 1)); the default w=2
        gives inverse cubic angstroms.

        Returns an independent NumPy float64 array copy, or None before
        fft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def chir_imag(self) -> NDArray[np.float64] | None:
        """Imaginary component of the complex forward transform, paired with r().

        Its sign follows the negative-exponent forward FFT, without an extra
        phase rotation. For dimensionless chi and integer kweight w, units are
        angstrom**(-(w + 1)); the default w=2 gives inverse cubic angstroms.
        Keep the complex components when phase matters.

        Returns an independent NumPy float64 array copy, or None before
        fft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def q(self) -> NDArray[np.float64] | None:
        """Back-transform q axis in inverse angstroms, paired with chiq().

        q describes the same physical variable as k, but labels a reconstructed
        signal after R filtering. Its spacing is determined by the inverse FFT
        length and the input R spacing; qmax_out limits the returned range.

        Returns an independent NumPy float64 array copy, or None before
        ifft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
    def chiq(self) -> NDArray[np.float64] | None:
        """Real R-filtered signal on q(), retaining the forward weighting and window.

        This is generally not the original unweighted chi(k). For dimensionless
        input chi, integer forward kweight w and inverse rweight v, its units
        are angstrom**(v - w); defaults w=2 and v=0 give inverse square
        angstroms. A selected R window removes contributions outside its taper.

        Returns an independent NumPy float64 array copy, or None before
        ifft() (or a dependent stage) succeeds or after invalidation.
        Reading this result does not run processing; editing the copy does
        not change the spectrum."""
