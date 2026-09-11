/** Rust-compatible option names. Constructors use the recommended Rust defaults. */
export type FFTGrid = "Input" | "Larch";
export type FTWindow = "Hanning" | "Parzen" | "Welch" | "Gaussian" | "Sine" | "KaiserBessel" | "FHanning";
export type AUTOBKSolver = "TrustRegionDogLeg" | "LegacyLm" | "LinearDirect";
export type AUTOBKClampScalePolicy = "FixedPenalty" | "Fixed" | "TwoPass";

/** Pre/post-edge normalization settings. Defaults adapt to the measured energy range. */
export interface PrePostEdgeOptions {
  /** Pre-edge fit start relative to E0, in eV. Default: infer from the measured range. */
  pre_edge_start?: number | undefined;
  /** Pre-edge fit end relative to E0, in eV. Default: infer from the pre-edge start. */
  pre_edge_end?: number | undefined;
  /** Post-edge fit start relative to E0, in eV. Default: infer from the available range (at most 25 eV). */
  norm_start?: number | undefined;
  /** Post-edge fit end relative to E0, in eV. Default: measured upper energy limit. */
  norm_end?: number | undefined;
  /** Post-edge polynomial degree, 0 through 5. Default: 0, 1 or 2 for fit spans below 50, below 350, or at least 350 eV. */
  norm_polyorder?: number | undefined;
  /** Victoreen energy exponent for the pre-edge fit. Default: 0. */
  n_victoreen?: number | undefined;
  /** Edge energy in eV. Default: detect from the spectrum. */
  e0?: number | undefined;
  /** Absorption edge-step override in mu units. Default: estimate from the fitted baselines. */
  edge_step?: number | undefined;
}
/** Pre/post-edge normalization settings. Defaults adapt to the measured energy range. Setters copy configurations; call free() when done. */
export class PrePostEdge {
  /** Create settings, optionally overriding Rust defaults. */
  constructor(options?: PrePostEdgeOptions);
  /** Release native memory. Do not use the object afterwards. */
  free(): void;
  /** Pre-edge fit start relative to E0, in eV. Default: infer from the measured range. */
  pre_edge_start: number | undefined;
  /** Pre-edge fit end relative to E0, in eV. Default: infer from the pre-edge start. */
  pre_edge_end: number | undefined;
  /** Post-edge fit start relative to E0, in eV. Default: infer from the available range (at most 25 eV). */
  norm_start: number | undefined;
  /** Post-edge fit end relative to E0, in eV. Default: measured upper energy limit. */
  norm_end: number | undefined;
  /** Post-edge polynomial degree, 0 through 5. Default: 0, 1 or 2 for fit spans below 50, below 350, or at least 350 eV. */
  norm_polyorder: number | undefined;
  /** Victoreen energy exponent for the pre-edge fit. Default: 0. */
  n_victoreen: number | undefined;
  /** Edge energy in eV. Default: detect from the spectrum. */
  e0: number | undefined;
  /** Absorption edge-step override in mu units. Default: estimate from the fitted baselines. */
  edge_step: number | undefined;
}

/** AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001. */
export interface AUTOBKOptions {
  /** Edge energy in eV. Default: use normalization E0. */
  ek0?: number | undefined;
  /** Background cutoff in angstroms. AUTOBK suppresses Fourier residuals below this R. Default: 1.0; increasing it can remove structural signal. */
  rbkg?: number | undefined;
  /** Spline knot count. Default: determine from rbkg and the k range. */
  nknots?: number | undefined;
  /** Background fit lower k limit in inverse angstroms. Default: 0.0. */
  kmin?: number | undefined;
  /** Background fit upper k limit in inverse angstroms. Default: available data limit. */
  kmax?: number | undefined;
  /** Uniform output k spacing in inverse angstroms. Default: 0.05. */
  kstep?: number | undefined;
  /** Number of samples at each endpoint used by the clamp. Default: 3; 0 disables clamping. */
  nclamp?: number | undefined;
  /** Low-k endpoint weight. Default: 0 (disabled). */
  clamp_lo?: number | undefined;
  /** High-k endpoint weight. Default: 1. */
  clamp_hi?: number | undefined;
  /** FixedPenalty strength. Recommended default: 0.001; 0 disables the endpoint penalty. */
  clamp_lambda?: number | undefined;
  /** FFT length for background removal. Default: 2048. */
  nfft?: number | undefined;
  /** Power of k used in the background objective. Default: 1. */
  kweight?: number | undefined;
  /** Background window taper width in inverse angstroms. Default: 0.1. */
  dk?: number | undefined;
  /** Legacy direct-solver ridge strength. Default: 0.0001; unused by FixedPenalty. */
  linear_regularization?: number | undefined;
  /** Maximum accepted linear-system condition number. Default: 1e8. */
  linear_condition_limit?: number | undefined;
  /** Legacy direct-solver residual acceptance ratio. Default: 1.05; unused by FixedPenalty. */
  linear_residual_ratio_limit?: number | undefined;
  /** Allow legacy solver fallback. Default: true; FixedPenalty never falls back. */
  linear_fallback_to_lm?: boolean | undefined;
  /** Reuse compatible spline/FFT geometry and SVD factors. Default: true; each spectrum has a new right-hand side and solution. */
  linear_workspace_cache?: boolean | undefined;
  /** Background Fourier window. Default: Hanning. */
  window?: FTWindow | undefined;
  /** Background solver. Recommended default: LinearDirect, required by FixedPenalty. TrustRegionDogLeg requires a Rust build with trust-region (included in Python, unavailable in Wasm). */
  solver?: AUTOBKSolver | undefined;
  /** Legacy fallback solver. Default: TrustRegionDogLeg in Python, LegacyLm in Wasm; unused by FixedPenalty. */
  linear_fallback_solver?: AUTOBKSolver | undefined;
  /** Endpoint model. Recommended default: FixedPenalty with LinearDirect; Fixed and TwoPass are legacy models. */
  clamp_scale_policy?: AUTOBKClampScalePolicy | undefined;
}
/** AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001. Setters copy configurations; call free() when done. */
export class AUTOBK {
  /** Create settings, optionally overriding Rust defaults. */
  constructor(options?: AUTOBKOptions);
  /** Release native memory. Do not use the object afterwards. */
  free(): void;
  /** Edge energy in eV. Default: use normalization E0. */
  ek0: number | undefined;
  /** Background cutoff in angstroms. AUTOBK suppresses Fourier residuals below this R. Default: 1.0; increasing it can remove structural signal. */
  rbkg: number | undefined;
  /** Spline knot count. Default: determine from rbkg and the k range. */
  nknots: number | undefined;
  /** Background fit lower k limit in inverse angstroms. Default: 0.0. */
  kmin: number | undefined;
  /** Background fit upper k limit in inverse angstroms. Default: available data limit. */
  kmax: number | undefined;
  /** Uniform output k spacing in inverse angstroms. Default: 0.05. */
  kstep: number | undefined;
  /** Number of samples at each endpoint used by the clamp. Default: 3; 0 disables clamping. */
  nclamp: number | undefined;
  /** Low-k endpoint weight. Default: 0 (disabled). */
  clamp_lo: number | undefined;
  /** High-k endpoint weight. Default: 1. */
  clamp_hi: number | undefined;
  /** FixedPenalty strength. Recommended default: 0.001; 0 disables the endpoint penalty. */
  clamp_lambda: number | undefined;
  /** FFT length for background removal. Default: 2048. */
  nfft: number | undefined;
  /** Power of k used in the background objective. Default: 1. */
  kweight: number | undefined;
  /** Background window taper width in inverse angstroms. Default: 0.1. */
  dk: number | undefined;
  /** Legacy direct-solver ridge strength. Default: 0.0001; unused by FixedPenalty. */
  linear_regularization: number | undefined;
  /** Maximum accepted linear-system condition number. Default: 1e8. */
  linear_condition_limit: number | undefined;
  /** Legacy direct-solver residual acceptance ratio. Default: 1.05; unused by FixedPenalty. */
  linear_residual_ratio_limit: number | undefined;
  /** Allow legacy solver fallback. Default: true; FixedPenalty never falls back. */
  linear_fallback_to_lm: boolean | undefined;
  /** Reuse compatible spline/FFT geometry and SVD factors. Default: true; each spectrum has a new right-hand side and solution. */
  linear_workspace_cache: boolean | undefined;
  /** Background Fourier window. Default: Hanning. */
  window: FTWindow | undefined;
  /** Background solver. Recommended default: LinearDirect, required by FixedPenalty. TrustRegionDogLeg requires a Rust build with trust-region (included in Python, unavailable in Wasm). */
  solver: AUTOBKSolver | undefined;
  /** Legacy fallback solver. Default: TrustRegionDogLeg in Python, LegacyLm in Wasm; unused by FixedPenalty. */
  linear_fallback_solver: AUTOBKSolver | undefined;
  /** Endpoint model. Recommended default: FixedPenalty with LinearDirect; Fixed and TwoPass are legacy models. */
  clamp_scale_policy: AUTOBKClampScalePolicy | undefined;
}

/** Forward Fourier-transform settings. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel window. */
export interface XrayFFTFOptions {
  /** Sampling/window domain. Default: Input (existing k grid). Larch resamples on the extended FFT window grid. */
  grid?: FFTGrid;
  /** Maximum displayed R in angstroms. Default: 10.0; does not truncate the inverse-transform filter. */
  rmax_out?: number | undefined;
  /** Low-k taper width in inverse angstroms. Default: 1.0. */
  dk?: number | undefined;
  /** High-k taper width in inverse angstroms. Default: use dk. */
  dk2?: number | undefined;
  /** Lower Fourier window limit in inverse angstroms. Default: 2.0; undefined uses the first k sample. */
  kmin?: number | undefined;
  /** Upper Fourier window limit in inverse angstroms. Default: 15.0; undefined uses the last k sample. */
  kmax?: number | undefined;
  /** Power of k applied before FFT. Default: 2.0; nonnegative values are floored to an integer. */
  kweight?: number | undefined;
  /** Forward FFT length. Default: 2048. */
  nfft?: number | undefined;
  /** FFT k spacing in inverse angstroms. Default: infer from input k. */
  kstep?: number | undefined;
  /** Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning. */
  window?: FTWindow | undefined;
}
/** Forward Fourier-transform settings. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel window. Setters copy configurations; call free() when done. */
export class XrayFFTF {
  /** Create settings, optionally overriding Rust defaults. */
  constructor(options?: XrayFFTFOptions);
  /** Release native memory. Do not use the object afterwards. */
  free(): void;
  /** Sampling/window domain. Default: Input (existing k grid). Larch resamples on the extended FFT window grid. */
  grid: FFTGrid;
  /** Maximum displayed R in angstroms. Default: 10.0; does not truncate the inverse-transform filter. */
  rmax_out: number | undefined;
  /** Low-k taper width in inverse angstroms. Default: 1.0. */
  dk: number | undefined;
  /** High-k taper width in inverse angstroms. Default: use dk. */
  dk2: number | undefined;
  /** Lower Fourier window limit in inverse angstroms. Default: 2.0; undefined uses the first k sample. */
  kmin: number | undefined;
  /** Upper Fourier window limit in inverse angstroms. Default: 15.0; undefined uses the last k sample. */
  kmax: number | undefined;
  /** Power of k applied before FFT. Default: 2.0; nonnegative values are floored to an integer. */
  kweight: number | undefined;
  /** Forward FFT length. Default: 2048. */
  nfft: number | undefined;
  /** FFT k spacing in inverse angstroms. Default: infer from input k. */
  kstep: number | undefined;
  /** Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning. */
  window: FTWindow | undefined;
}

/** Inverse Fourier-transform settings. Set rmin/rmax to select an R-space shell. */
export interface XrayFFTROptions {
  /** Maximum back-transform q in inverse angstroms. Default: 10.0. */
  qmax_out?: number | undefined;
  /** Low-R taper width in angstroms. Default: 1.0. */
  dr?: number | undefined;
  /** High-R taper width in angstroms. Default: use dr. */
  dr2?: number | undefined;
  /** Lower inverse-transform window limit in angstroms. Default: 0.0. */
  rmin?: number | undefined;
  /** Upper inverse-transform window limit in angstroms. Default: 20.0; choose a shell range for R filtering. */
  rmax?: number | undefined;
  /** Power of R applied before IFFT. Default: 0.0; nonnegative values are floored to an integer. */
  rweight?: number | undefined;
  /** Inverse FFT length. Default: 2048; leave kstep automatic when changing this. */
  nfft?: number | undefined;
  /** Output q spacing in inverse angstroms. Default: infer from input R and nfft; an explicit value must match that spacing. */
  kstep?: number | undefined;
  /** Inverse Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning. */
  window?: FTWindow | undefined;
}
/** Inverse Fourier-transform settings. Set rmin/rmax to select an R-space shell. Setters copy configurations; call free() when done. */
export class XrayFFTR {
  /** Create settings, optionally overriding Rust defaults. */
  constructor(options?: XrayFFTROptions);
  /** Release native memory. Do not use the object afterwards. */
  free(): void;
  /** Maximum back-transform q in inverse angstroms. Default: 10.0. */
  qmax_out: number | undefined;
  /** Low-R taper width in angstroms. Default: 1.0. */
  dr: number | undefined;
  /** High-R taper width in angstroms. Default: use dr. */
  dr2: number | undefined;
  /** Lower inverse-transform window limit in angstroms. Default: 0.0. */
  rmin: number | undefined;
  /** Upper inverse-transform window limit in angstroms. Default: 20.0; choose a shell range for R filtering. */
  rmax: number | undefined;
  /** Power of R applied before IFFT. Default: 0.0; nonnegative values are floored to an integer. */
  rweight: number | undefined;
  /** Inverse FFT length. Default: 2048; leave kstep automatic when changing this. */
  nfft: number | undefined;
  /** Output q spacing in inverse angstroms. Default: infer from input R and nfft; an explicit value must match that spacing. */
  kstep: number | undefined;
  /** Inverse Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning. */
  window: FTWindow | undefined;
}

export class NormalizationMethod {
  private constructor();
  /** Release native memory. Do not use the object afterwards. */
  free(): void;
  /** Copy pre/post-edge settings into a normalization method. */
  static PrePostEdge(parameters: PrePostEdge): NormalizationMethod;
  /** Create automatic pre/post-edge normalization settings. */
  static new_prepostedge(): NormalizationMethod;
  /** Create an unimplemented MBack placeholder; processing raises an error. */
  static new_mback(): NormalizationMethod;
}
export class BackgroundMethod {
  private constructor();
  /** Release native memory. Do not use the object afterwards. */
  free(): void;
  /** Copy AUTOBK settings into a background method. */
  static AUTOBK(parameters: AUTOBK): BackgroundMethod;
  /** Create the recommended default AUTOBK method. */
  static new_autobk(): BackgroundMethod;
  /** Create an unimplemented ILPBkg placeholder; processing raises an error. */
  static new_ilpbkg(): BackgroundMethod;
}

/** Mutable Rust spectrum. Stages run synchronously and return the same object.
 * Missing prerequisites use the selected algorithms and their defaults.
 * Array getters return independent copies, or undefined before their stage runs.
 */
export class Spectrum {
  /** Copy energy (eV) and absorption mu into a spectrum. Inputs must be finite, one-dimensional, equal-length, with strictly increasing energy. */
  constructor(energy: Float64Array, mu: Float64Array);
  /** Create a spectrum from energy (eV) and absorption mu. Copies input arrays; equivalent to the constructor. */
  static from_arrays(energy: Float64Array, mu: Float64Array): Spectrum;
  /** Release native memory. Do not use the object afterwards. */
  free(): void;
  /** Replace energy (eV) and mu, copy the inputs and clear E0 and derived results. Returns this spectrum. */
  set_spectrum(energy: Float64Array, mu: Float64Array): this;
  /** Set edge energy in eV and invalidate normalization and downstream results. Returns this spectrum. */
  set_e0(e0: number): this;
  /** Copy normalization settings and invalidate normalization and downstream results. Accepts PrePostEdge directly or a NormalizationMethod; omitted/undefined restores automatic pre/post-edge normalization. */
  set_normalization_method(method?: PrePostEdge | NormalizationMethod | null): this;
  /** Copy background settings and invalidate background and downstream results. Accepts AUTOBK directly or a BackgroundMethod; omitted/undefined restores default AUTOBK. */
  set_background_method(method?: AUTOBK | BackgroundMethod | null): this;
  /** Copy inverse-transform settings; clear q and chi(q) while preserving forward results. Returns this spectrum. */
  set_ifft(parameters: XrayFFTR): this;
  /** Copy forward-transform settings; clear Fourier and inverse results while preserving normalization and chi(k). Returns this spectrum. */
  set_fft(parameters: XrayFFTF): this;
  /** Edge energy in eV, or undefined before detection or assignment. */
  e0(): number | undefined;
  /** Detect edge energy from mu and invalidate dependent results. Returns this spectrum. */
  find_e0(): this;
  /** Run pre/post-edge normalization, finding E0 if needed. Returns this spectrum. */
  normalize(): this;
  /** Run AUTOBK, computing missing normalization first. Returns this spectrum. */
  calc_background(): this;
  /** Compute chi(R), running missing normalization and AUTOBK first. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048. Returns this spectrum. */
  fft(): this;
  /** Back-transform chi(R) to chi(q), running missing forward stages first. Configure the R window with set_ifft(new XrayFFTR(...)). Returns this spectrum. */
  ifft(): this;
  /** Clear all calculated results while retaining stage settings for recomputation. Returns this spectrum. */
  invalidate_derived(): this;
  /** Uniform background k axis in inverse angstroms; pairs with chi(). Returns an independent array copy, or undefined before its stage runs. */
  k(): Float64Array | undefined;
  /** Unweighted EXAFS chi(k) = (mu - smooth background) / edge_step; dimensionless and paired with k(). Returns an independent array copy, or undefined before its stage runs. */
  chi(): Float64Array | undefined;
  /** Normalized absorption (mu - pre_edge) / edge_step on the input energy grid; dimensionless. Returns an independent array copy, or undefined before its stage runs. */
  norm(): Float64Array | undefined;
  /** Normalized absorption with its fitted post-edge trend removed, preserving the edge value; dimensionless. Returns an independent array copy, or undefined before its stage runs. */
  flat(): Float64Array | undefined;
  /** Fitted pre-edge baseline in mu units on the input energy grid. Returns an independent array copy, or undefined before its stage runs. */
  pre_edge(): Float64Array | undefined;
  /** Fitted post-edge baseline in mu units on the input energy grid. Returns an independent array copy, or undefined before its stage runs. */
  post_edge(): Float64Array | undefined;
  /** Forward-transform R axis in angstroms; pairs with chir_mag/real/imag(). Peaks are not phase-corrected bond lengths. Returns an independent array copy, or undefined before its stage runs. */
  r(): Float64Array | undefined;
  /** Forward Fourier window values; use kwin_k() for the matching axis. Returns an independent array copy, or undefined before its stage runs. */
  kwin(): Float64Array | undefined;
  /** Forward Fourier window k axis in inverse angstroms; may differ from k() with grid=Larch. Returns an independent array copy, or undefined before its stage runs. */
  kwin_k(): Float64Array | undefined;
  /** Magnitude of chi(R); pairs with r(). Returns an independent array copy, or undefined before its stage runs. */
  chir_mag(): Float64Array | undefined;
  /** Real component of chi(R); pairs with r(). Returns an independent array copy, or undefined before its stage runs. */
  chir_real(): Float64Array | undefined;
  /** Imaginary component of chi(R); pairs with r(). Returns an independent array copy, or undefined before its stage runs. */
  chir_imag(): Float64Array | undefined;
  /** Back-transform q axis in inverse angstroms; pairs with chiq(). Returns an independent array copy, or undefined before its stage runs. */
  q(): Float64Array | undefined;
  /** Real R-filtered signal on q(); forward k-weighting and windowing remain, so this is not generally the unweighted chi(k). Returns an independent array copy, or undefined before its stage runs. */
  chiq(): Float64Array | undefined;
}
