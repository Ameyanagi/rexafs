"""Typed Rust spectrum processing. Energy: eV; k/q: inverse angstroms; R: angstroms."""

from typing import Literal, TypeAlias

import numpy as np
from numpy.typing import ArrayLike, NDArray

from . import io as io

__version__: str

FFTGrid: TypeAlias = Literal["Input", "Larch"]
FTWindow: TypeAlias = Literal[
    "Hanning", "Parzen", "Welch", "Gaussian", "Sine", "KaiserBessel", "FHanning"
]
AUTOBKSolver: TypeAlias = Literal["TrustRegionDogLeg", "LegacyLm", "LinearDirect"]
AUTOBKClampScalePolicy: TypeAlias = Literal["FixedPenalty", "Fixed", "TwoPass"]

class PrePostEdge:
    """Pre/post-edge normalization settings. Defaults adapt to the measured energy range. Settings are copied when assigned to a spectrum."""
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
        """Create settings; override only the parameters you need. Defaults match Rust new()."""
    pre_edge_start: float | None
    """Pre-edge fit start relative to E0, in eV. Default: infer from the measured range."""
    pre_edge_end: float | None
    """Pre-edge fit end relative to E0, in eV. Default: infer from the pre-edge start."""
    norm_start: float | None
    """Post-edge fit start relative to E0, in eV. Default: infer from the available range (at most 25 eV)."""
    norm_end: float | None
    """Post-edge fit end relative to E0, in eV. Default: measured upper energy limit."""
    norm_polyorder: int | None
    """Post-edge polynomial degree, 0 through 5. Default: 0, 1 or 2 for fit spans below 50, below 350, or at least 350 eV."""
    n_victoreen: int | None
    """Victoreen energy exponent for the pre-edge fit. Default: 0."""
    e0: float | None
    """Edge energy in eV. Default: detect from the spectrum."""
    edge_step: float | None
    """Absorption edge-step override in mu units. Default: estimate from the fitted baselines."""

class AUTOBK:
    """AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001. Settings are copied when assigned to a spectrum."""
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
        """Create settings; override only the parameters you need. Defaults match Rust new()."""
    ek0: float | None
    """Edge energy in eV. Default: use normalization E0."""
    rbkg: float | None
    """Background cutoff in angstroms. AUTOBK suppresses Fourier residuals below this R. Default: 1.0; increasing it can remove structural signal."""
    nknots: int | None
    """Spline knot count. Default: determine from rbkg and the k range."""
    kmin: float | None
    """Background fit lower k limit in inverse angstroms. Default: 0.0."""
    kmax: float | None
    """Background fit upper k limit in inverse angstroms. Default: available data limit."""
    kstep: float | None
    """Uniform output k spacing in inverse angstroms. Default: 0.05."""
    nclamp: int | None
    """Number of samples at each endpoint used by the clamp. Default: 3; 0 disables clamping."""
    clamp_lo: int | None
    """Low-k endpoint weight. Default: 0 (disabled)."""
    clamp_hi: int | None
    """High-k endpoint weight. Default: 1."""
    clamp_lambda: float | None
    """FixedPenalty strength. Recommended default: 0.001; 0 disables the endpoint penalty."""
    nfft: int | None
    """FFT length for background removal. Default: 2048."""
    kweight: int | None
    """Power of k used in the background objective. Default: 1."""
    dk: float | None
    """Background window taper width in inverse angstroms. Default: 0.1."""
    linear_regularization: float | None
    """Legacy direct-solver ridge strength. Default: 0.0001; unused by FixedPenalty."""
    linear_condition_limit: float | None
    """Maximum accepted linear-system condition number. Default: 1e8."""
    linear_residual_ratio_limit: float | None
    """Legacy direct-solver residual acceptance ratio. Default: 1.05; unused by FixedPenalty."""
    linear_fallback_to_lm: bool | None
    """Allow legacy solver fallback. Default: True; FixedPenalty never falls back."""
    linear_workspace_cache: bool | None
    """Reuse compatible spline/FFT geometry and SVD factors. Default: True; each spectrum has a new right-hand side and solution."""
    window: FTWindow | None
    """Background Fourier window. Default: Hanning."""
    solver: AUTOBKSolver | None
    """Background solver. Recommended default: LinearDirect, required by FixedPenalty. TrustRegionDogLeg requires a Rust build with trust-region (included in Python, unavailable in Wasm)."""
    linear_fallback_solver: AUTOBKSolver | None
    """Legacy fallback solver. Default: TrustRegionDogLeg in Python, LegacyLm in Wasm; unused by FixedPenalty."""
    clamp_scale_policy: AUTOBKClampScalePolicy | None
    """Endpoint model. Recommended default: FixedPenalty with LinearDirect; Fixed and TwoPass are legacy models."""

class XrayFFTF:
    """Forward Fourier-transform settings. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel window. Settings are copied when assigned to a spectrum."""
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
        """Create settings; override only the parameters you need. Defaults match Rust new()."""
    grid: FFTGrid
    """Sampling/window domain. Default: Input (existing k grid). Larch resamples on the extended FFT window grid."""
    rmax_out: float | None
    """Maximum displayed R in angstroms. Default: 10.0; does not truncate the inverse-transform filter."""
    dk: float | None
    """Low-k taper width in inverse angstroms. Default: 1.0."""
    dk2: float | None
    """High-k taper width in inverse angstroms. Default: use dk."""
    kmin: float | None
    """Lower Fourier window limit in inverse angstroms. Default: 2.0; None/undefined uses the first k sample."""
    kmax: float | None
    """Upper Fourier window limit in inverse angstroms. Default: 15.0; None/undefined uses the last k sample."""
    kweight: float | None
    """Power of k applied before FFT. Default: 2.0; nonnegative values are floored to an integer."""
    nfft: int | None
    """Forward FFT length. Default: 2048."""
    kstep: float | None
    """FFT k spacing in inverse angstroms. Default: infer from input k."""
    window: FTWindow | None
    """Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning."""

class XrayFFTR:
    """Inverse Fourier-transform settings. Set rmin/rmax to select an R-space shell. Settings are copied when assigned to a spectrum."""
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
        """Create settings; override only the parameters you need. Defaults match Rust new()."""
    qmax_out: float | None
    """Maximum back-transform q in inverse angstroms. Default: 10.0."""
    dr: float | None
    """Low-R taper width in angstroms. Default: 1.0."""
    dr2: float | None
    """High-R taper width in angstroms. Default: use dr."""
    rmin: float | None
    """Lower inverse-transform window limit in angstroms. Default: 0.0."""
    rmax: float | None
    """Upper inverse-transform window limit in angstroms. Default: 20.0; choose a shell range for R filtering."""
    rweight: float | None
    """Power of R applied before IFFT. Default: 0.0; nonnegative values are floored to an integer."""
    nfft: int | None
    """Inverse FFT length. Default: 2048; leave kstep automatic when changing this."""
    kstep: float | None
    """Output q spacing in inverse angstroms. Default: infer from input R and nfft; an explicit value must match that spacing."""
    window: FTWindow | None
    """Inverse Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning."""

class NormalizationMethod:
    """Rust algorithm selection. Prefer passing settings directly to the spectrum setter."""
    @staticmethod
    def PrePostEdge(parameters: PrePostEdge) -> NormalizationMethod:
        """Copy pre/post-edge settings into a normalization method."""
    @staticmethod
    def new_prepostedge() -> NormalizationMethod:
        """Create automatic pre/post-edge normalization settings."""
    @staticmethod
    def new_mback() -> NormalizationMethod:
        """Create an unimplemented MBack placeholder; processing raises ValueError."""

class BackgroundMethod:
    """Rust algorithm selection. Prefer passing settings directly to the spectrum setter."""
    @staticmethod
    def AUTOBK(parameters: AUTOBK) -> BackgroundMethod:
        """Copy AUTOBK settings into a background method."""
    @staticmethod
    def new_autobk() -> BackgroundMethod:
        """Create the recommended default AUTOBK method."""
    @staticmethod
    def new_ilpbkg() -> BackgroundMethod:
        """Create an unimplemented ILPBkg placeholder; processing raises RuntimeError."""

class Spectrum:
    """Mutable Rust spectrum. Stages return the same object and release the GIL.

    Invalid inputs raise ValueError; background/FFT failures raise RuntimeError.
    Missing prerequisite stages use the configured methods and Rust defaults.
    """
    def __init__(self, energy: ArrayLike, mu: ArrayLike) -> None:
        """Copy energy (eV) and absorption mu into a spectrum. Inputs must be finite, one-dimensional, equal-length, with strictly increasing energy."""
    @staticmethod
    def from_arrays(energy: ArrayLike, mu: ArrayLike) -> Spectrum:
        """Create a spectrum from energy (eV) and absorption mu. Copies input arrays; equivalent to the constructor."""
    def set_spectrum(self, energy: ArrayLike, mu: ArrayLike) -> Spectrum:
        """Replace energy (eV) and mu, copy the inputs and clear E0 and derived results. Returns this spectrum."""
    def set_e0(self, e0: float) -> Spectrum:
        """Set edge energy in eV and invalidate normalization and downstream results. Returns this spectrum."""
    def set_normalization_method(
        self, method: PrePostEdge | NormalizationMethod | None = None
    ) -> Spectrum:
        """Copy normalization settings and invalidate normalization and downstream results. Accepts PrePostEdge directly or a NormalizationMethod; omitted/None restores automatic pre/post-edge normalization."""
    def set_background_method(
        self, method: AUTOBK | BackgroundMethod | None = None
    ) -> Spectrum:
        """Copy background settings and invalidate background and downstream results. Accepts AUTOBK directly or a BackgroundMethod; omitted/None restores default AUTOBK."""
    def set_ifft(self, parameters: XrayFFTR) -> Spectrum:
        """Copy inverse-transform settings; clear q and chi(q) while preserving forward results. Returns this spectrum."""
    def set_fft(self, parameters: XrayFFTF) -> Spectrum:
        """Copy forward-transform settings; clear Fourier and inverse results while preserving normalization and chi(k). Returns this spectrum."""
    def e0(self) -> float | None:
        """Edge energy in eV, or None before detection or assignment."""
    def find_e0(self) -> Spectrum:
        """Detect edge energy from mu and invalidate dependent results. Returns this spectrum."""
    def normalize(self) -> Spectrum:
        """Run pre/post-edge normalization, finding E0 if needed. Returns this spectrum."""
    def calc_background(self) -> Spectrum:
        """Run AUTOBK, computing missing normalization first. Returns this spectrum."""
    def fft(self) -> Spectrum:
        """Compute chi(R), running missing normalization and AUTOBK first. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048. Returns this spectrum."""
    def ifft(self) -> Spectrum:
        """Back-transform chi(R) to chi(q), running missing forward stages first. Configure the R window with set_ifft(XrayFFTR(...)). Returns this spectrum."""
    def invalidate_derived(self) -> Spectrum:
        """Clear all calculated results while retaining stage settings for recomputation. Returns this spectrum."""
    def k(self) -> NDArray[np.float64] | None:
        """Uniform background k axis in inverse angstroms; pairs with chi(). Returns an independent array copy, or None before its stage runs."""
    def chi(self) -> NDArray[np.float64] | None:
        """Unweighted EXAFS chi(k) = (mu - smooth background) / edge_step; dimensionless and paired with k(). Returns an independent array copy, or None before its stage runs."""
    def norm(self) -> NDArray[np.float64] | None:
        """Normalized absorption (mu - pre_edge) / edge_step on the input energy grid; dimensionless. Returns an independent array copy, or None before its stage runs."""
    def flat(self) -> NDArray[np.float64] | None:
        """Normalized absorption with its fitted post-edge trend removed, preserving the edge value; dimensionless. Returns an independent array copy, or None before its stage runs."""
    def pre_edge(self) -> NDArray[np.float64] | None:
        """Fitted pre-edge baseline in mu units on the input energy grid. Returns an independent array copy, or None before its stage runs."""
    def post_edge(self) -> NDArray[np.float64] | None:
        """Fitted post-edge baseline in mu units on the input energy grid. Returns an independent array copy, or None before its stage runs."""
    def r(self) -> NDArray[np.float64] | None:
        """Forward-transform R axis in angstroms; pairs with chir_mag/real/imag(). Peaks are not phase-corrected bond lengths. Returns an independent array copy, or None before its stage runs."""
    def kwin(self) -> NDArray[np.float64] | None:
        """Forward Fourier window values; use kwin_k() for the matching axis. Returns an independent array copy, or None before its stage runs."""
    def kwin_k(self) -> NDArray[np.float64] | None:
        """Forward Fourier window k axis in inverse angstroms; may differ from k() with grid=Larch. Returns an independent array copy, or None before its stage runs."""
    def chir_mag(self) -> NDArray[np.float64] | None:
        """Magnitude of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs."""
    def chir_real(self) -> NDArray[np.float64] | None:
        """Real component of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs."""
    def chir_imag(self) -> NDArray[np.float64] | None:
        """Imaginary component of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs."""
    def q(self) -> NDArray[np.float64] | None:
        """Back-transform q axis in inverse angstroms; pairs with chiq(). Returns an independent array copy, or None before its stage runs."""
    def chiq(self) -> NDArray[np.float64] | None:
        """Real R-filtered signal on q(); forward k-weighting and windowing remain, so this is not generally the unweighted chi(k). Returns an independent array copy, or None before its stage runs."""
