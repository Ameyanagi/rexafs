/**
 * Forward-transform sampling convention. Input preserves the background k grid; Larch
 * constructs a zero-origin uniform grid with linear resampling and an extended window domain.
 * The default is Input. This choice affects Fourier preparation only, not the background
 * k()/chi() arrays.
 * Names are case-sensitive. Assigning an unsupported grid throws a string exception and
 * leaves the existing selection unchanged.
 */
export type FFTGrid = "Input" | "Larch";
/**
 * Supported Fourier-window families. Hanning and FHanning use cosine tapers; Parzen, Welch,
 * Gaussian, Sine and KaiserBessel use their named shapes. Forward and inverse settings default
 * to KaiserBessel; AUTOBK defaults to Hanning. Parameters dk/dr have shape-dependent meaning,
 * and KaiserBessel also uses them as shape parameters. See the [window
 * implementation](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/xafsutils.rs)
 * for the exact formulas.
 * Names are case-sensitive. An unsupported name throws an Error immediately during property
 * assignment, leaving the existing window selection unchanged.
 */
export type FTWindow = "Hanning" | "Parzen" | "Welch" | "Gaussian" | "Sine" | "KaiserBessel" | "FHanning";
/**
 * Background spline solver. LinearDirect is the recommended default and is required for
 * FixedPenalty. LegacyLm is the iterative Levenberg-Marquardt solver for legacy objectives.
 * TrustRegionDogLeg requires a native Rust feature and is unavailable in the distributed Wasm
 * build; selecting it throws during processing.
 * An unrecognized name throws an Error immediately during property assignment and leaves the
 * existing selection unchanged.
 */
export type AUTOBKSolver = "TrustRegionDogLeg" | "LegacyLm" | "LinearDirect";
/**
 * Background endpoint model. FixedPenalty is the recommended fixed mean-square endpoint
 * penalty, controlled by clamp_lambda and solved by LinearDirect. Fixed and TwoPass retain
 * historical residual-dependent scaling behavior. Selecting a policy changes the optimization
 * objective, not merely the numerical solver.
 * Names are case-sensitive; an unsupported name throws an Error during property assignment
 * without changing the existing selection.
 */
export type AUTOBKClampScalePolicy = "FixedPenalty" | "Fixed" | "TwoPass";

/**
 * Named field overrides for new PrePostEdge(options). Omitted fields retain constructor
 * defaults; explicitly assigning undefined uses the resolution described for each field. An
 * unknown key or a non-object options argument throws TypeError. Numerical range and data-
 * dependent checks generally run when the configured processing stage executes.
 */
export interface PrePostEdgeOptions {
  /**
   * Lower pre-edge fit bound, as an energy offset from E0 in eV. Default: undefined selects a
   * rounded estimate near the beginning of the measured range. Choose a region below the
   * absorption edge without other edges or glitches; this region determines the subtracted
   * baseline.
   */
  pre_edge_start?: number | undefined;
  /**
   * Upper pre-edge fit bound relative to E0, in eV. Default: undefined derives a rounded value
   * from pre_edge_start. The pre-edge region is normally below E0 (negative offsets); moving it
   * toward the edge can include near-edge structure in the baseline fit.
   */
  pre_edge_end?: number | undefined;
  /**
   * Lower post-edge fit bound relative to E0, in eV. Default: undefined derives it from
   * norm_end, normally capped at 25 eV and kept at least 10 eV below norm_end. This region
   * estimates the absorption edge step and the trend removed by flat().
   */
  norm_start?: number | undefined;
  /**
   * Upper post-edge fit bound relative to E0, in eV. Default: undefined rounds the available
   * post-edge extent to a nearby 5 eV boundary, without extending beyond the measured range.
   * Choose the range before a second absorption edge if one is present.
   */
  norm_end?: number | undefined;
  /**
   * Integer degree of the polynomial fitted to pre-edge-subtracted absorption above E0.
   * Default: undefined selects 0, 1 or 2 for post-edge fit spans below 50, below 350, or at
   * least 350 eV. Values are clamped to 0 through 5. Higher order follows more curvature but
   * can absorb spectral structure.
   */
  norm_polyorder?: number | undefined;
  /**
   * Integer energy exponent used to model the pre-edge baseline. Default: undefined resolves to
   * 0, a straight-line fit to mu(E). For exponent n, the code fits a line to mu(E) * E^n and
   * divides the fitted line by E^n; E is energy in eV. Use 0 unless this additional energy
   * dependence is justified.
   */
  n_victoreen?: number | undefined;
  /**
   * Absorption-edge energy in eV. Default: undefined uses the spectrum E0 if assigned,
   * otherwise derivative-based edge detection. E0 sets the origin of the fit offsets and the
   * subsequent energy-to-k conversion. A supplied value must be finite and strictly between
   * the measured energy endpoints; spectrum processing throws an Error otherwise.
   */
  e0?: number | undefined;
  /**
   * Absorption step used as the normalization divisor, in the same units as mu. Default:
   * undefined estimates post_edge - pre_edge at the input sample nearest E0. Use a positive
   * physical step; the implementation floors finite values below 1e-12 to 1e-12 and rejects
   * nonfinite steps. Changing it rescales norm() and chi().
   */
  edge_step?: number | undefined;
}
/**
 * Settings for absorption-edge normalization and flattening.
 *
 * The pre-edge fit estimates the smooth baseline before the edge. The post-edge fit estimates
 * the edge step and the trend used by flat(). All energy bounds are offsets from E0 in eV, not
 * absolute energies. Undefined fields select data-dependent values where described; inspect the
 * measured fit regions before interpreting a normalized spectrum.
 *
 * Settings are copied when assigned to a spectrum. Changing this object afterwards
 * requires assigning it again; free() releases only this object's Wasm allocation.
 *
 * Background and normalization conventions are explained in [Newville, Fundamentals of
 * XAFS](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf). Automatic range and
 * polynomial rules are rexafs implementation choices in
 * [PrePostEdge](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/normalization.rs).
 */
export class PrePostEdge {
  /**
   * Create owned settings with the recommended defaults described below. Browser callers must
   * await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
   * and free() this object when finished.
   *
   * Automatic fields are resolved on the spectrum's copy during processing; resolved values
   * are not written back into the original settings object. Resolved values stay in the
   * spectrum's settings until those settings are replaced.
   *
   * Stable 0.2.4 uses this constructor without arguments, followed by property assignment. Next
   * (the source checkout) also accepts the named options object.
   */
  constructor(options?: PrePostEdgeOptions);
  /**
   * Release this object's Wasm allocation. Do not call methods, read fields or free it again
   * afterwards. Arrays and settings already copied elsewhere remain valid.
   */
  free(): void;
  /**
   * Lower pre-edge fit bound, as an energy offset from E0 in eV. Default: undefined selects a
   * rounded estimate near the beginning of the measured range. Choose a region below the
   * absorption edge without other edges or glitches; this region determines the subtracted
   * baseline.
   */
  pre_edge_start: number | undefined;
  /**
   * Upper pre-edge fit bound relative to E0, in eV. Default: undefined derives a rounded value
   * from pre_edge_start. The pre-edge region is normally below E0 (negative offsets); moving it
   * toward the edge can include near-edge structure in the baseline fit.
   */
  pre_edge_end: number | undefined;
  /**
   * Lower post-edge fit bound relative to E0, in eV. Default: undefined derives it from
   * norm_end, normally capped at 25 eV and kept at least 10 eV below norm_end. This region
   * estimates the absorption edge step and the trend removed by flat().
   */
  norm_start: number | undefined;
  /**
   * Upper post-edge fit bound relative to E0, in eV. Default: undefined rounds the available
   * post-edge extent to a nearby 5 eV boundary, without extending beyond the measured range.
   * Choose the range before a second absorption edge if one is present.
   */
  norm_end: number | undefined;
  /**
   * Integer degree of the polynomial fitted to pre-edge-subtracted absorption above E0.
   * Default: undefined selects 0, 1 or 2 for post-edge fit spans below 50, below 350, or at
   * least 350 eV. Values are clamped to 0 through 5. Higher order follows more curvature but
   * can absorb spectral structure.
   */
  norm_polyorder: number | undefined;
  /**
   * Integer energy exponent used to model the pre-edge baseline. Default: undefined resolves to
   * 0, a straight-line fit to mu(E). For exponent n, the code fits a line to mu(E) * E^n and
   * divides the fitted line by E^n; E is energy in eV. Use 0 unless this additional energy
   * dependence is justified.
   */
  n_victoreen: number | undefined;
  /**
   * Absorption-edge energy in eV. Default: undefined uses the spectrum E0 if assigned,
   * otherwise derivative-based edge detection. E0 sets the origin of the fit offsets and the
   * subsequent energy-to-k conversion. A supplied value must be finite and strictly between
   * the measured energy endpoints; spectrum processing throws an Error otherwise.
   */
  e0: number | undefined;
  /**
   * Absorption step used as the normalization divisor, in the same units as mu. Default:
   * undefined estimates post_edge - pre_edge at the input sample nearest E0. Use a positive
   * physical step; the implementation floors finite values below 1e-12 to 1e-12 and rejects
   * nonfinite steps. Changing it rescales norm() and chi().
   */
  edge_step: number | undefined;
}

/**
 * Named field overrides for new AUTOBK(options). Omitted fields retain constructor defaults;
 * explicitly assigning undefined uses the resolution described for each field. An unknown key
 * or a non-object options argument throws TypeError. Numerical range and data-dependent checks
 * generally run when the configured processing stage executes.
 */
export interface AUTOBKOptions {
  /**
   * Edge energy used for the energy-to-k conversion, in eV. Default: undefined uses
   * normalization E0. An in-range override changes the background k grid without redefining the
   * pre/post-edge fitting regions. Prefer a consistent E0 across both stages; an out-of-range
   * override is discarded.
   */
  ek0?: number | undefined;
  /**
   * Background cutoff in angstroms. Default: 1.0; undefined restores this default. AUTOBK
   * minimizes low-R Fourier components of the residual below a cutoff derived from this value.
   * Increasing rbkg allows a more flexible background and can remove real first-shell signal;
   * start below the first structural peak.
   */
  rbkg?: number | undefined;
  /**
   * Number of spline control points used to represent the smooth background. Default: undefined
   * derives the count from rbkg and the fitted k interval; the resolved count is clamped to 5
   * through 128. More points add flexibility and can overfit structure. This is not the length
   * of the repeated endpoint knot vector.
   */
  nknots?: number | undefined;
  /**
   * Lower background fit/window bound in inverse angstroms. Default: 0.0; undefined restores
   * this default. Require kmin < the usable kmax. Raising this bound excludes the near-edge
   * region from the Fourier objective; the returned k() grid still begins at zero.
   */
  kmin?: number | undefined;
  /**
   * Upper background fit/window bound in inverse angstroms. Default: undefined uses the
   * available data limit and explicit values are capped at that limit. Reducing it can exclude
   * noisy high-k data and shortens the returned k()/chi() arrays.
   */
  kmax?: number | undefined;
  /**
   * Spacing of the uniform output k() grid in inverse angstroms. Default: 0.05; undefined
   * restores this default. Must be finite and positive. Smaller spacing interpolates the same
   * measured data more densely and changes the internal R sampling; it does not add
   * experimental resolution.
   */
  kstep?: number | undefined;
  /**
   * Number of samples at each enabled endpoint used to discourage large residual chi values.
   * Default: 3; undefined restores this default. FixedPenalty requires a nonnegative integer,
   * caps the count at the available samples, and includes the last high-k sample. Use 0 to
   * disable endpoint clamping.
   */
  nclamp?: number | undefined;
  /**
   * Integer multiplier for the low-k endpoint residuals. Default: 0, which disables that
   * endpoint. In FixedPenalty the absolute value multiplies each residual, so its square
   * weights the objective. The active low-k samples begin at k=0 on the output grid.
   */
  clamp_lo?: number | undefined;
  /**
   * Integer multiplier for the high-k endpoint residuals. Default: 1. In FixedPenalty the
   * absolute value multiplies each residual, so doubling it quadruples that endpoint
   * contribution before averaging. Use 0 to disable the high-k endpoint penalty.
   */
  clamp_hi?: number | undefined;
  /**
   * Numerical strength of the FixedPenalty endpoint term. Recommended default: 0.001; undefined
   * restores this default. The objective adds lambda times the mean squared active, weighted
   * endpoint chi residual to the mean squared low-R residual. Require a finite nonnegative
   * value; 0 disables the endpoint term.
   *
   * This empirical balance is tied to the implemented residual convention: the FixedPenalty
   * Fourier residual uses the fixed numerical factor 0.05/sqrt(pi), while the endpoint residual
   * uses unweighted, edge-step-normalized chi. Changing kweight or the window changes the
   * balance at a fixed lambda. The parameter is unused by legacy endpoint policies; it is not a
   * universal physical constant. See [the fixed-penalty
   * objective](https://rexafs.com/docs/science/autobk/).
   */
  clamp_lambda?: number | undefined;
  /**
   * FFT length used inside background removal. Default: 2048; undefined restores this default.
   * Use a positive integer large enough to contain the prepared k grid. It sets the internal R
   * spacing together with kstep; increasing zero-padding refines that grid without adding
   * measured information.
   */
  nfft?: number | undefined;
  /**
   * Integer power of k applied in the background Fourier objective. Default: 1; undefined
   * restores this default. Larger powers emphasize high-k residuals and their noise. The
   * returned chi() remains unweighted; the independent XrayFFTF.kweight controls the later
   * displayed transform.
   */
  kweight?: number | undefined;
  /**
   * Background window parameter. Default: 0.1; undefined restores this default. For the default
   * Hanning window it controls endpoint taper widths in inverse angstroms. KaiserBessel also
   * uses this numeric value as a dimensionless shape parameter; equal dk does not make
   * different window families equivalent.
   */
  dk?: number | undefined;
  /**
   * Ridge strength for the legacy LinearDirect objective. Default: 0.0001; undefined restores
   * this default. Unused by the recommended FixedPenalty model, which solves the specified
   * endpoint-penalized least-squares problem without this additional ridge term. Change only
   * when reproducing a legacy calculation.
   */
  linear_regularization?: number | undefined;
  /**
   * Largest accepted condition number of the linear system. Default: 1e8; undefined restores
   * this default. FixedPenalty applies this limit to the column-scaled design matrix and
   * requires a finite value of at least 1. An ill-conditioned or rank-deficient solve throws
   * instead of silently changing the objective.
   */
  linear_condition_limit?: number | undefined;
  /**
   * Acceptance threshold for comparing the legacy direct solution residual against its
   * reference residual. Default: 1.05; undefined restores this default. Unused by FixedPenalty.
   * This is a numerical fallback criterion, not a statistical uncertainty or goodness-of-fit
   * probability.
   */
  linear_residual_ratio_limit?: number | undefined;
  /**
   * Allow the legacy direct solver to retry with linear_fallback_solver if its checks fail.
   * Default: true; undefined restores this default. Despite the historical name, the chosen
   * fallback need not be Levenberg-Marquardt. FixedPenalty never falls back and ignores this
   * flag.
   */
  linear_fallback_to_lm?: boolean | undefined;
  /**
   * Reuse compatible spline geometry, Fourier operators and matrix factorization. Default:
   * true; undefined restores this default. Each spectrum still supplies new data and receives a
   * new solution, with condition checks repeated. Disabling this changes reuse and runtime, not
   * the specified objective.
   */
  linear_workspace_cache?: boolean | undefined;
  /**
   * Window family used inside the background Fourier objective. Default: Hanning; undefined
   * restores Hanning. The taper reduces ringing at the selected k boundaries. This setting is
   * independent of the later XrayFFTF window; see FTWindow for the accepted names.
   */
  window?: FTWindow | undefined;
  /**
   * Algorithm used to determine background spline coefficients. Recommended default:
   * LinearDirect, required by FixedPenalty; undefined restores it. LegacyLm solves the legacy
   * objective iteratively. TrustRegionDogLeg requires a native Rust feature and throws when
   * processing in the published Wasm package.
   */
  solver?: AUTOBKSolver | undefined;
  /**
   * Solver used only when an enabled legacy LinearDirect fallback is needed. Default in Wasm:
   * LegacyLm; undefined also resolves to LegacyLm with the default fallback flag. FixedPenalty
   * ignores this option. TrustRegionDogLeg is unavailable in Wasm, and LinearDirect cannot be
   * its own fallback.
   */
  linear_fallback_solver?: AUTOBKSolver | undefined;
  /**
   * Endpoint penalty model. Recommended default: FixedPenalty; undefined restores it.
   * FixedPenalty uses a fixed mean-square endpoint term and requires LinearDirect. Fixed and
   * TwoPass preserve older residual-dependent scaling choices for reproducing historical
   * results; they are different objectives.
   */
  clamp_scale_policy?: AUTOBKClampScalePolicy | undefined;
}
/**
 * Settings for extracting extended X-ray absorption fine structure, chi(k), with a cubic
 * spline background in photoelectron wavenumber k.
 *
 * AUTOBK separates slowly varying atomic absorption from oscillations associated with
 * neighboring atoms by suppressing low-R Fourier residuals. Recommended starting values are
 * rbkg=1 angstrom, kstep=0.05 inverse angstroms, kweight=1, Hanning, and LinearDirect with
 * FixedPenalty and clamp_lambda=0.001. Choose rbkg below structural signal; excessive
 * background flexibility can remove that signal.
 *
 * The original method is [Newville et al. (1993)](https://doi.org/10.1103/PhysRevB.47.14126).
 * The fixed endpoint penalty and its default strength are rexafs-specific choices, implemented
 * in
 * [background/fixed.rs](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/background/fixed.rs);
 * they are not part of that paper's objective.
 *
 * The spectrum copies assigned background settings. Reassign after editing, and release your
 * settings with free() when finished. A processing failure, including an ill-conditioned
 * fixed-penalty fit, throws an Error without silently switching objectives.
 */
export class AUTOBK {
  /**
   * Create owned settings with the recommended defaults described below. Browser callers must
   * await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
   * and free() this object when finished.
   *
   * Automatic fields are resolved on the spectrum's copy during processing; resolved values
   * are not written back into the original settings object. Resolved scalar defaults and ek0
   * are retained in the spectrum. Automatic kmax and nknots remain unset in stored settings
   * and are calculated locally for each input.
   *
   * Stable 0.2.4 uses this constructor without arguments, followed by property assignment. Next
   * (the source checkout) also accepts the named options object.
   */
  constructor(options?: AUTOBKOptions);
  /**
   * Release this object's Wasm allocation. Do not call methods, read fields or free it again
   * afterwards. Arrays and settings already copied elsewhere remain valid.
   */
  free(): void;
  /**
   * Edge energy used for the energy-to-k conversion, in eV. Default: undefined uses
   * normalization E0. An in-range override changes the background k grid without redefining the
   * pre/post-edge fitting regions. Prefer a consistent E0 across both stages; an out-of-range
   * override is discarded.
   */
  ek0: number | undefined;
  /**
   * Background cutoff in angstroms. Default: 1.0; undefined restores this default. AUTOBK
   * minimizes low-R Fourier components of the residual below a cutoff derived from this value.
   * Increasing rbkg allows a more flexible background and can remove real first-shell signal;
   * start below the first structural peak.
   */
  rbkg: number | undefined;
  /**
   * Number of spline control points used to represent the smooth background. Default: undefined
   * derives the count from rbkg and the fitted k interval; the resolved count is clamped to 5
   * through 128. More points add flexibility and can overfit structure. This is not the length
   * of the repeated endpoint knot vector.
   */
  nknots: number | undefined;
  /**
   * Lower background fit/window bound in inverse angstroms. Default: 0.0; undefined restores
   * this default. Require kmin < the usable kmax. Raising this bound excludes the near-edge
   * region from the Fourier objective; the returned k() grid still begins at zero.
   */
  kmin: number | undefined;
  /**
   * Upper background fit/window bound in inverse angstroms. Default: undefined uses the
   * available data limit and explicit values are capped at that limit. Reducing it can exclude
   * noisy high-k data and shortens the returned k()/chi() arrays.
   */
  kmax: number | undefined;
  /**
   * Spacing of the uniform output k() grid in inverse angstroms. Default: 0.05; undefined
   * restores this default. Must be finite and positive. Smaller spacing interpolates the same
   * measured data more densely and changes the internal R sampling; it does not add
   * experimental resolution.
   */
  kstep: number | undefined;
  /**
   * Number of samples at each enabled endpoint used to discourage large residual chi values.
   * Default: 3; undefined restores this default. FixedPenalty requires a nonnegative integer,
   * caps the count at the available samples, and includes the last high-k sample. Use 0 to
   * disable endpoint clamping.
   */
  nclamp: number | undefined;
  /**
   * Integer multiplier for the low-k endpoint residuals. Default: 0, which disables that
   * endpoint. In FixedPenalty the absolute value multiplies each residual, so its square
   * weights the objective. The active low-k samples begin at k=0 on the output grid.
   */
  clamp_lo: number | undefined;
  /**
   * Integer multiplier for the high-k endpoint residuals. Default: 1. In FixedPenalty the
   * absolute value multiplies each residual, so doubling it quadruples that endpoint
   * contribution before averaging. Use 0 to disable the high-k endpoint penalty.
   */
  clamp_hi: number | undefined;
  /**
   * Numerical strength of the FixedPenalty endpoint term. Recommended default: 0.001; undefined
   * restores this default. The objective adds lambda times the mean squared active, weighted
   * endpoint chi residual to the mean squared low-R residual. Require a finite nonnegative
   * value; 0 disables the endpoint term.
   *
   * This empirical balance is tied to the implemented residual convention: the FixedPenalty
   * Fourier residual uses the fixed numerical factor 0.05/sqrt(pi), while the endpoint residual
   * uses unweighted, edge-step-normalized chi. Changing kweight or the window changes the
   * balance at a fixed lambda. The parameter is unused by legacy endpoint policies; it is not a
   * universal physical constant. See [the fixed-penalty
   * objective](https://rexafs.com/docs/science/autobk/).
   */
  clamp_lambda: number | undefined;
  /**
   * FFT length used inside background removal. Default: 2048; undefined restores this default.
   * Use a positive integer large enough to contain the prepared k grid. It sets the internal R
   * spacing together with kstep; increasing zero-padding refines that grid without adding
   * measured information.
   */
  nfft: number | undefined;
  /**
   * Integer power of k applied in the background Fourier objective. Default: 1; undefined
   * restores this default. Larger powers emphasize high-k residuals and their noise. The
   * returned chi() remains unweighted; the independent XrayFFTF.kweight controls the later
   * displayed transform.
   */
  kweight: number | undefined;
  /**
   * Background window parameter. Default: 0.1; undefined restores this default. For the default
   * Hanning window it controls endpoint taper widths in inverse angstroms. KaiserBessel also
   * uses this numeric value as a dimensionless shape parameter; equal dk does not make
   * different window families equivalent.
   */
  dk: number | undefined;
  /**
   * Ridge strength for the legacy LinearDirect objective. Default: 0.0001; undefined restores
   * this default. Unused by the recommended FixedPenalty model, which solves the specified
   * endpoint-penalized least-squares problem without this additional ridge term. Change only
   * when reproducing a legacy calculation.
   */
  linear_regularization: number | undefined;
  /**
   * Largest accepted condition number of the linear system. Default: 1e8; undefined restores
   * this default. FixedPenalty applies this limit to the column-scaled design matrix and
   * requires a finite value of at least 1. An ill-conditioned or rank-deficient solve throws
   * instead of silently changing the objective.
   */
  linear_condition_limit: number | undefined;
  /**
   * Acceptance threshold for comparing the legacy direct solution residual against its
   * reference residual. Default: 1.05; undefined restores this default. Unused by FixedPenalty.
   * This is a numerical fallback criterion, not a statistical uncertainty or goodness-of-fit
   * probability.
   */
  linear_residual_ratio_limit: number | undefined;
  /**
   * Allow the legacy direct solver to retry with linear_fallback_solver if its checks fail.
   * Default: true; undefined restores this default. Despite the historical name, the chosen
   * fallback need not be Levenberg-Marquardt. FixedPenalty never falls back and ignores this
   * flag.
   */
  linear_fallback_to_lm: boolean | undefined;
  /**
   * Reuse compatible spline geometry, Fourier operators and matrix factorization. Default:
   * true; undefined restores this default. Each spectrum still supplies new data and receives a
   * new solution, with condition checks repeated. Disabling this changes reuse and runtime, not
   * the specified objective.
   */
  linear_workspace_cache: boolean | undefined;
  /**
   * Window family used inside the background Fourier objective. Default: Hanning; undefined
   * restores Hanning. The taper reduces ringing at the selected k boundaries. This setting is
   * independent of the later XrayFFTF window; see FTWindow for the accepted names.
   */
  window: FTWindow | undefined;
  /**
   * Algorithm used to determine background spline coefficients. Recommended default:
   * LinearDirect, required by FixedPenalty; undefined restores it. LegacyLm solves the legacy
   * objective iteratively. TrustRegionDogLeg requires a native Rust feature and throws when
   * processing in the published Wasm package.
   */
  solver: AUTOBKSolver | undefined;
  /**
   * Solver used only when an enabled legacy LinearDirect fallback is needed. Default in Wasm:
   * LegacyLm; undefined also resolves to LegacyLm with the default fallback flag. FixedPenalty
   * ignores this option. TrustRegionDogLeg is unavailable in Wasm, and LinearDirect cannot be
   * its own fallback.
   */
  linear_fallback_solver: AUTOBKSolver | undefined;
  /**
   * Endpoint penalty model. Recommended default: FixedPenalty; undefined restores it.
   * FixedPenalty uses a fixed mean-square endpoint term and requires LinearDirect. Fixed and
   * TwoPass preserve older residual-dependent scaling choices for reproducing historical
   * results; they are different objectives.
   */
  clamp_scale_policy: AUTOBKClampScalePolicy | undefined;
}

/**
 * Named field overrides for new XrayFFTF(options). Omitted fields retain constructor defaults;
 * explicitly assigning undefined uses the resolution described for each field. An unknown key
 * or a non-object options argument throws TypeError. Numerical range and data-dependent checks
 * generally run when the configured processing stage executes.
 */
export interface XrayFFTFOptions {
  /**
   * Sampling and window-construction convention. Default: Input preserves the prepared
   * background k grid. Larch linearly resamples onto a zero-origin grid and extends the window
   * domain as needed. Neither changes the returned background k()/chi(); always pair kwin()
   * with kwin_k().
   */
  grid?: FFTGrid;
  /**
   * Maximum reported R in angstroms. Default: 10.0; undefined restores this default. This
   * limits the r() and chir_*() output arrays, not the internally retained Fourier bins used by
   * ifft(). It does not change the transform amplitude or frequency resolution. ifft()
   * nevertheless needs at least two reported R samples to infer/validate their spacing, so
   * rmax_out=0 is insufficient for a back-transform.
   */
  rmax_out?: number | undefined;
  /**
   * Low-k window parameter. Default: 1.0; undefined restores this default. For taper windows it
   * controls transition geometry in inverse angstroms. KaiserBessel also uses the same numeric
   * value as a dimensionless shape parameter, so it cannot be compared as a universal taper
   * width across all windows.
   */
  dk?: number | undefined;
  /**
   * High-k window parameter. Default: undefined uses dk. It controls the upper transition
   * geometry in inverse angstroms; interpretation depends on the window family. Use the same
   * value as dk for symmetric endpoint settings.
   */
  dk2?: number | undefined;
  /**
   * Lower Fourier window bound in inverse angstroms. Default: 2.0; explicitly assigning
   * undefined uses the first prepared k sample. Both bounds must be finite with kmin < kmax;
   * negative bounds are accepted. Select this above the region where the EXAFS approximation or
   * background subtraction is unreliable.
   */
  kmin?: number | undefined;
  /**
   * Upper Fourier window bound in inverse angstroms. Default: 15.0; explicitly assigning
   * undefined uses the last prepared k sample. Require kmax > kmin. Select this within the
   * useful measured range; high-k noise can dominate after k weighting.
   */
  kmax?: number | undefined;
  /**
   * Power of k applied before the forward transform. Default: 2.0; undefined restores this
   * default. Finite nonnegative values are floored to an integer w. Larger w emphasizes high-k
   * oscillations and noise; for dimensionless chi, the transformed amplitude has units
   * angstrom^(-(w + 1)).
   */
  kweight?: number | undefined;
  /**
   * Forward FFT length N. Default: 2048; undefined restores this default. Require an integer of
   * at least 2. The R spacing is pi / (N * kstep), in angstroms. Use N at least as large as the
   * prepared data: Input truncates excess samples, whereas Larch rejects a window grid that
   * does not fit. Larger zero-padding does not improve experimental resolution.
   */
  nfft?: number | undefined;
  /**
   * k spacing used to scale the transform and label R, in inverse angstroms. Default: undefined
   * infers the first spacing of the prepared k grid (normally 0.05 from AUTOBK defaults). Larch
   * also uses this spacing to resample chi. Input does not resample, so keep it consistent with
   * the input grid. Require a finite positive value. The inferred value is retained in the
   * spectrum's copied FFT settings. After changing the background grid, reassign FFT settings
   * with kstep undefined to infer the new spacing.
   */
  kstep?: number | undefined;
  /**
   * Fourier window family. Constructor default: KaiserBessel; explicitly assigning undefined
   * selects the window routine's Hanning fallback. The window reduces truncation ringing and
   * changes amplitude. No correction for window area or coherent gain is applied; see FTWindow
   * for valid names.
   */
  window?: FTWindow | undefined;
}
/**
 * Settings for converting weighted, windowed chi(k) into complex chi(R).
 *
 * Recommended starting values: kmin=2, kmax=15 inverse angstroms, kweight=2, KaiserBessel,
 * dk=1 and nfft=2048. Adapt the k interval to the useful measured data. The default Input grid
 * preserves the background grid; automatic kstep normally resolves to AUTOBK's 0.05 inverse
 * angstroms.
 *
 * For a uniform zero-origin grid `k[j]=j*kstep`, prepare
 * `g[j] = chi(k[j]) * k[j]^w * window[j]`. The code computes `chiR[m] = (kstep /
 * sqrt(pi)) * sum_j g[j] * exp(-2*pi*i*j*m/N)`, with N=nfft, w=kweight, i^2=-1 and
 * `R[m]=pi*m/(N*kstep)`. Here j indexes prepared k samples and m indexes nonnegative
 * Fourier bins. The sum uses the first N prepared samples and zeros for missing
 * samples. There is no 1/N forward normalization or window-area correction. For dimensionless
 * chi, chi(R) has units angstrom^(-(w+1)).
 *
 * This is the [NumPy unnormalized forward DFT
 * convention](https://numpy.org/doc/stable/reference/routines.fft.html#implementation-details)
 * with the explicit factor in
 * [xftf_fast_nalgebra](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/xrayfft.rs).
 * Scattering phases shift the peaks, so R is not automatically a phase-corrected bond
 * distance; see [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).
 *
 * set_fft() copies settings. Reassign after editing, and free() this object when finished.
 */
export class XrayFFTF {
  /**
   * Create owned settings with the recommended defaults described below. Browser callers must
   * await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
   * and free() this object when finished.
   *
   * Automatic fields are resolved on the spectrum's copy during processing; resolved values
   * are not written back into the original settings object. Resolved values stay in the
   * spectrum's settings until those settings are replaced.
   *
   * Stable 0.2.4 uses this constructor without arguments, followed by property assignment. Next
   * (the source checkout) also accepts the named options object.
   */
  constructor(options?: XrayFFTFOptions);
  /**
   * Release this object's Wasm allocation. Do not call methods, read fields or free it again
   * afterwards. Arrays and settings already copied elsewhere remain valid.
   */
  free(): void;
  /**
   * Sampling and window-construction convention. Default: Input preserves the prepared
   * background k grid. Larch linearly resamples onto a zero-origin grid and extends the window
   * domain as needed. Neither changes the returned background k()/chi(); always pair kwin()
   * with kwin_k().
   */
  grid: FFTGrid;
  /**
   * Maximum reported R in angstroms. Default: 10.0; undefined restores this default. This
   * limits the r() and chir_*() output arrays, not the internally retained Fourier bins used by
   * ifft(). It does not change the transform amplitude or frequency resolution. ifft()
   * nevertheless needs at least two reported R samples to infer/validate their spacing, so
   * rmax_out=0 is insufficient for a back-transform.
   */
  rmax_out: number | undefined;
  /**
   * Low-k window parameter. Default: 1.0; undefined restores this default. For taper windows it
   * controls transition geometry in inverse angstroms. KaiserBessel also uses the same numeric
   * value as a dimensionless shape parameter, so it cannot be compared as a universal taper
   * width across all windows.
   */
  dk: number | undefined;
  /**
   * High-k window parameter. Default: undefined uses dk. It controls the upper transition
   * geometry in inverse angstroms; interpretation depends on the window family. Use the same
   * value as dk for symmetric endpoint settings.
   */
  dk2: number | undefined;
  /**
   * Lower Fourier window bound in inverse angstroms. Default: 2.0; explicitly assigning
   * undefined uses the first prepared k sample. Both bounds must be finite with kmin < kmax;
   * negative bounds are accepted. Select this above the region where the EXAFS approximation or
   * background subtraction is unreliable.
   */
  kmin: number | undefined;
  /**
   * Upper Fourier window bound in inverse angstroms. Default: 15.0; explicitly assigning
   * undefined uses the last prepared k sample. Require kmax > kmin. Select this within the
   * useful measured range; high-k noise can dominate after k weighting.
   */
  kmax: number | undefined;
  /**
   * Power of k applied before the forward transform. Default: 2.0; undefined restores this
   * default. Finite nonnegative values are floored to an integer w. Larger w emphasizes high-k
   * oscillations and noise; for dimensionless chi, the transformed amplitude has units
   * angstrom^(-(w + 1)).
   */
  kweight: number | undefined;
  /**
   * Forward FFT length N. Default: 2048; undefined restores this default. Require an integer of
   * at least 2. The R spacing is pi / (N * kstep), in angstroms. Use N at least as large as the
   * prepared data: Input truncates excess samples, whereas Larch rejects a window grid that
   * does not fit. Larger zero-padding does not improve experimental resolution.
   */
  nfft: number | undefined;
  /**
   * k spacing used to scale the transform and label R, in inverse angstroms. Default: undefined
   * infers the first spacing of the prepared k grid (normally 0.05 from AUTOBK defaults). Larch
   * also uses this spacing to resample chi. Input does not resample, so keep it consistent with
   * the input grid. Require a finite positive value. The inferred value is retained in the
   * spectrum's copied FFT settings. After changing the background grid, reassign FFT settings
   * with kstep undefined to infer the new spacing.
   */
  kstep: number | undefined;
  /**
   * Fourier window family. Constructor default: KaiserBessel; explicitly assigning undefined
   * selects the window routine's Hanning fallback. The window reduces truncation ringing and
   * changes amplitude. No correction for window area or coherent gain is applied; see FTWindow
   * for valid names.
   */
  window: FTWindow | undefined;
}

/**
 * Named field overrides for new XrayFFTR(options). Omitted fields retain constructor defaults;
 * explicitly assigning undefined uses the resolution described for each field. An unknown key
 * or a non-object options argument throws TypeError. Numerical range and data-dependent checks
 * generally run when the configured processing stage executes.
 */
export interface XrayFFTROptions {
  /**
   * Largest reported back-transform q in inverse angstroms. Default: 10.0; undefined restores
   * this default. This crops q()/chiq() to the available output range without changing the
   * inverse calculation. Require a finite nonnegative value.
   */
  qmax_out?: number | undefined;
  /**
   * Low-R window parameter. Default: 1.0; undefined restores this default. For taper windows it
   * controls transition geometry in angstroms. KaiserBessel also uses this numeric value as a
   * dimensionless shape parameter; different windows do not have equivalent shape for the same
   * dr.
   */
  dr?: number | undefined;
  /**
   * High-R window parameter. Default: undefined uses dr. It controls the upper transition
   * geometry in angstroms, with interpretation depending on the selected window family. Use
   * matching dr and dr2 for symmetric endpoint settings.
   */
  dr2?: number | undefined;
  /**
   * Lower inverse-transform window bound in angstroms. Default: 0.0; explicitly assigning
   * undefined uses the first input R sample. Require 0 <= rmin < rmax. A nonzero lower bound
   * can exclude low-R background contributions.
   */
  rmin?: number | undefined;
  /**
   * Upper inverse-transform window bound in angstroms. Default: 20.0; explicitly assigning
   * undefined uses the last reported input R sample. Require rmax > rmin. Choose the R interval
   * around the shell contribution of interest; uncorrected Fourier peaks are not directly bond
   * lengths.
   */
  rmax?: number | undefined;
  /**
   * Power of R applied before the inverse transform. Default: 0.0; undefined restores this
   * default. Finite nonnegative values are floored to an integer. Leave at 0 for ordinary shell
   * filtering; a positive value additionally emphasizes larger-R contributions and changes the
   * signal units.
   */
  rweight?: number | undefined;
  /**
   * Inverse FFT length N. Default: 2048; undefined restores this default. Require an integer of
   * at least 2. Keeping kstep automatic resolves output spacing from the existing R grid and N;
   * reducing N discards high-R bins and increasing it pads them with zeros.
   */
  nfft?: number | undefined;
  /**
   * Output q spacing in inverse angstroms. Default: undefined computes pi / (nfft * delta_R),
   * where delta_R is the input R spacing in angstroms. An explicit value must agree with that
   * relationship or processing throws. Leave automatic when changing nfft. The resolved value
   * is retained in the spectrum's copy. After changing the forward R grid, reassign inverse
   * settings whose kstep is undefined to resolve it again.
   */
  kstep?: number | undefined;
  /**
   * R-space window family. Constructor default: KaiserBessel; explicitly assigning undefined
   * selects Hanning. The window selects and tapers the R contributions retained in chiq(). The
   * forward k weight and window are not divided out by the inverse transform.
   */
  window?: FTWindow | undefined;
}
/**
 * Settings for filtering selected R-space contributions and returning a real signal chi(q).
 *
 * Choose rmin/rmax in angstroms to enclose the contributions of interest. Defaults are rmin=0,
 * rmax=20, dr=1, KaiserBessel, rweight=0, nfft=2048 and qmax_out=10 inverse angstroms.
 * Automatic kstep preserves consistency with the input R spacing.
 *
 * rexafs windows and optionally R-weights the complex forward bins, supplies their conjugate
 * negative-frequency partners, and performs a real inverse DFT with scale
 * sqrt(pi)/(kstep*nfft). This implements a real filtered back-transform; forward k-weighting
 * and windowing remain in the result. It is not a general recovery of the original unweighted
 * chi(k), and is not Larch's complex, one-sided inverse representation. See
 * [inverse_fft.rs](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/inverse_fft.rs)
 * and the [DFT normalization
 * convention](https://numpy.org/doc/stable/reference/routines.fft.html#normalization).
 *
 * set_ifft() copies settings. Reassign after editing, and free() this object when finished.
 */
export class XrayFFTR {
  /**
   * Create owned settings with the recommended defaults described below. Browser callers must
   * await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
   * and free() this object when finished.
   *
   * Automatic fields are resolved on the spectrum's copy during processing; resolved values
   * are not written back into the original settings object. Resolved values stay in the
   * spectrum's settings until those settings are replaced.
   *
   * This class is available in Next (the source checkout), not npm 0.2.4.
   */
  constructor(options?: XrayFFTROptions);
  /**
   * Release this object's Wasm allocation. Do not call methods, read fields or free it again
   * afterwards. Arrays and settings already copied elsewhere remain valid.
   */
  free(): void;
  /**
   * Largest reported back-transform q in inverse angstroms. Default: 10.0; undefined restores
   * this default. This crops q()/chiq() to the available output range without changing the
   * inverse calculation. Require a finite nonnegative value.
   */
  qmax_out: number | undefined;
  /**
   * Low-R window parameter. Default: 1.0; undefined restores this default. For taper windows it
   * controls transition geometry in angstroms. KaiserBessel also uses this numeric value as a
   * dimensionless shape parameter; different windows do not have equivalent shape for the same
   * dr.
   */
  dr: number | undefined;
  /**
   * High-R window parameter. Default: undefined uses dr. It controls the upper transition
   * geometry in angstroms, with interpretation depending on the selected window family. Use
   * matching dr and dr2 for symmetric endpoint settings.
   */
  dr2: number | undefined;
  /**
   * Lower inverse-transform window bound in angstroms. Default: 0.0; explicitly assigning
   * undefined uses the first input R sample. Require 0 <= rmin < rmax. A nonzero lower bound
   * can exclude low-R background contributions.
   */
  rmin: number | undefined;
  /**
   * Upper inverse-transform window bound in angstroms. Default: 20.0; explicitly assigning
   * undefined uses the last reported input R sample. Require rmax > rmin. Choose the R interval
   * around the shell contribution of interest; uncorrected Fourier peaks are not directly bond
   * lengths.
   */
  rmax: number | undefined;
  /**
   * Power of R applied before the inverse transform. Default: 0.0; undefined restores this
   * default. Finite nonnegative values are floored to an integer. Leave at 0 for ordinary shell
   * filtering; a positive value additionally emphasizes larger-R contributions and changes the
   * signal units.
   */
  rweight: number | undefined;
  /**
   * Inverse FFT length N. Default: 2048; undefined restores this default. Require an integer of
   * at least 2. Keeping kstep automatic resolves output spacing from the existing R grid and N;
   * reducing N discards high-R bins and increasing it pads them with zeros.
   */
  nfft: number | undefined;
  /**
   * Output q spacing in inverse angstroms. Default: undefined computes pi / (nfft * delta_R),
   * where delta_R is the input R spacing in angstroms. An explicit value must agree with that
   * relationship or processing throws. Leave automatic when changing nfft. The resolved value
   * is retained in the spectrum's copy. After changing the forward R grid, reassign inverse
   * settings whose kstep is undefined to resolve it again.
   */
  kstep: number | undefined;
  /**
   * R-space window family. Constructor default: KaiserBessel; explicitly assigning undefined
   * selects Hanning. The window selects and tapers the R contributions retained in chiq(). The
   * forward k weight and window are not divided out by the inverse transform.
   */
  window: FTWindow | undefined;
}

/**
 * Select the normalization algorithm and hold an owned copy of its settings.
 *
 * Use PrePostEdge(settings) for customized pre/post-edge fits or new_prepostedge() for
 * automatic defaults. The MBack factory is only a placeholder; it does not implement that
 * algorithm. Copy this method into Spectrum.set_normalization_method(), then free() the wrapper
 * when no longer needed.
 */
export class NormalizationMethod {
  private constructor();
  /**
   * Release this object's Wasm allocation. Do not call methods, read fields or free it again
   * afterwards. Arrays and settings already copied elsewhere remain valid.
   */
  free(): void;
  /**
   * Copy the supplied pre/post-edge configuration into a new owned algorithm wrapper. The input
   * settings are not consumed. Assign the wrapper to Spectrum.set_normalization_method(), then
   * free() it when no longer needed; the spectrum retains its own copy.
   */
  static PrePostEdge(parameters: PrePostEdge): NormalizationMethod;
  /**
   * Create a new owned pre/post-edge normalization wrapper with automatic E0, fit ranges,
   * polynomial order and edge step. It does not process any spectrum. Assign it to
   * Spectrum.set_normalization_method() and free() the wrapper when finished.
   */
  static new_prepostedge(): NormalizationMethod;
  /**
   * Create an owned MBack placeholder for API compatibility. MBack processing is not
   * implemented and normalize() throws if this method is selected. Use new_prepostedge() for
   * supported normalization, and free() any placeholder you create.
   */
  static new_mback(): NormalizationMethod;
}
/**
 * Select the background-removal algorithm and hold an owned copy of its settings.
 *
 * Use AUTOBK(settings) for a configured spline background or new_autobk() for the recommended
 * defaults. The ILPBkg factory is only a placeholder; it does not implement that algorithm.
 * Copy this method into Spectrum.set_background_method(), then free() the wrapper when no
 * longer needed.
 */
export class BackgroundMethod {
  private constructor();
  /**
   * Release this object's Wasm allocation. Do not call methods, read fields or free it again
   * afterwards. Arrays and settings already copied elsewhere remain valid.
   */
  free(): void;
  /**
   * Copy the supplied AUTOBK configuration into a new owned algorithm wrapper. The input
   * settings are not consumed. Assign the wrapper to Spectrum.set_background_method(), then
   * free() it when no longer needed; the spectrum retains its own copy.
   */
  static AUTOBK(parameters: AUTOBK): BackgroundMethod;
  /**
   * Create a new owned AUTOBK wrapper with the recommended LinearDirect/FixedPenalty defaults,
   * rbkg=1 angstrom and clamp_lambda=0.001. It does not process any spectrum. Assign it to
   * Spectrum.set_background_method() and free() the wrapper when finished.
   */
  static new_autobk(): BackgroundMethod;
  /**
   * Create an owned ILPBkg placeholder for API compatibility. This background method is not
   * implemented and calc_background() throws if selected. Use new_autobk() for supported
   * background removal, and free() any placeholder you create.
   */
  static new_ilpbkg(): BackgroundMethod;
}

/**
 * Mutable absorption spectrum processed by the Rust engine in WebAssembly.
 *
 * Construct from finite, equal-length Float64Arrays containing strictly increasing photon
 * energy in eV and absorption mu. The supplied absorption scale is retained until edge-step
 * normalization. Arrays and stage settings are copied, so callers keep ownership of their
 * inputs.
 *
 * Stages run synchronously and return this same object for chaining. fft() computes missing
 * normalization and AUTOBK stages using the selected settings; ifft() additionally computes the
 * forward transform if absent. Changing or rerunning a stage clears dependent results. A stage
 * error throws; inspect or correct the inputs/settings before retrying.
 *
 * Result getters never run processing. They return independent Float64Array copies, or
 * undefined before their stage runs and after invalidation. Copies remain valid after free().
 * Browser callers must await init() before constructing objects, and should use a Web Worker
 * for long calculations. Release every spectrum and settings/wrapper object with free() when
 * finished.
 *
 * See [processing theory](https://rexafs.com/docs/science/processing/) for equations,
 * assumptions and interpretation.
 */
export class Spectrum {
  /**
   * Copy measured photon energy in eV and absorption mu into a new, initially unprocessed
   * spectrum. Both inputs must be Float64Arrays of equal length, with at least two finite
   * samples and strictly increasing energy. Duplicate or decreasing energies are rejected.
   * Throws TypeError for other array types, and Error for invalid data or uninitialized browser
   * Wasm. Processing may need more samples than construction; free() the spectrum when
   * finished.
   */
  constructor(energy: Float64Array, mu: Float64Array);
  /**
   * Create a new spectrum by copying measured energy in eV and absorption mu. Equivalent to new
   * Spectrum(energy, mu), including Float64Array type checks and the requirement for at least
   * two finite, strictly increasing energy samples of matching length. No processing runs
   * automatically at construction; free() the result when finished.
   */
  static from_arrays(energy: Float64Array, mu: Float64Array): Spectrum;
  /**
   * Release this object's Wasm allocation. Do not call methods, read fields or free it again
   * afterwards. Arrays and settings already copied elsewhere remain valid.
   */
  free(): void;
  /**
   * Replace measured energy (eV) and absorption mu with independent copies. Uses the same
   * validation as the constructor and preserves the previous spectrum if input validation
   * fails. Clears E0 and all calculated results while retaining stage settings. Returns this
   * spectrum; recompute the desired stages after replacement.
   */
  set_spectrum(energy: Float64Array, mu: Float64Array): this;
  /**
   * Assign a finite edge energy in eV. Clears normalization, background, forward and inverse
   * results, and updates the selected normalization/background energy origins. The value is not
   * checked against the measured range until processing. Throws TypeError for a non-number or
   * RangeError for a nonfinite value. Returns this spectrum.
   */
  set_e0(e0: number): this;
  /**
   * Copy the selected normalization method and clear normalization, background, forward and
   * inverse results. A specified method E0 overrides the spectrum E0; otherwise the existing E0
   * is retained. The caller keeps ownership of the settings and wrapper and may free them after
   * assignment. Later edits require reassignment. Returns this spectrum.
   *
   * Omitting the argument, undefined or null restores automatic pre/post-edge settings while
   * retaining the selected E0. These reset forms work in stable 0.2.4 and Next. For custom
   * settings, stable 0.2.4 accepts a NormalizationMethod wrapper; Next (the source checkout)
   * also accepts PrePostEdge directly.
   */
  set_normalization_method(method?: PrePostEdge | NormalizationMethod | null): this;
  /**
   * Copy the selected background method and clear background, forward and inverse results while
   * retaining normalization. The caller keeps ownership of the settings and wrapper and may
   * free them after assignment. Later edits require reassignment. Returns this spectrum.
   *
   * Omitting the argument, undefined or null restores default AUTOBK settings. These reset
   * forms work in stable 0.2.4 and Next. For custom settings, stable 0.2.4 accepts a
   * BackgroundMethod wrapper; Next (the source checkout) also accepts AUTOBK directly. This
   * does not reset forward or inverse configuration values that were already resolved
   * automatically.
   */
  set_background_method(method?: AUTOBK | BackgroundMethod | null): this;
  /**
   * Copy inverse-transform settings and clear q()/chiq() while preserving normalization,
   * background and forward results. Settings can be freed after assignment; later edits require
   * reassignment. Invalid R ranges or inconsistent kstep/nfft are reported when ifft() runs.
   * Returns this spectrum. Assign settings with kstep undefined to resolve spacing again after
   * changing the forward grid. This method and XrayFFTR are Next additions after 0.2.4.
   */
  set_ifft(parameters: XrayFFTR): this;
  /**
   * Copy forward-transform settings and clear r(), chir_*(), kwin(), kwin_k(), q() and chiq().
   * Normalization and background k()/chi() are preserved. Settings can be freed after
   * assignment; later edits require reassignment. Invalid numerical settings are reported when
   * fft() runs. Returns this spectrum. Assign settings with kstep undefined to request fresh
   * spacing inference, for example after changing AUTOBK.kstep. Inverse settings are retained;
   * if their spacing was already resolved, they may also need replacement before ifft().
   */
  set_fft(parameters: XrayFFTF): this;
  /**
   * Return the selected or detected absorption-edge energy in eV, or undefined before
   * assignment/detection. Reading this value does not detect an edge or run any processing.
   */
  e0(): number | undefined;
  /**
   * Estimate the absorption-edge energy from the derivative of measured mu(E), including local
   * smoothing/refinement. Clears normalization and all downstream results, then returns this
   * spectrum. Throws if the input cannot be used for edge detection. Inspect the result for
   * noisy, multiple-edge or unusual spectra and use set_e0() for an explicit choice.
   */
  find_e0(): this;
  /**
   * Fit the selected pre/post-edge model, find E0 if needed, and calculate dimensionless
   * norm()/flat() plus pre_edge()/post_edge() baselines on the original energy grid. Clears
   * background and all Fourier results even when normalization was already present. Uses
   * automatic PrePostEdge defaults if no method was selected. Throws on unsupported methods or
   * failed baseline fits. Returns this spectrum.
   */
  normalize(): this;
  /**
   * Fit the selected smooth background and calculate unweighted, dimensionless chi(k) on k().
   * Runs missing normalization first and uses default AUTOBK if no background method was
   * selected. Clears forward and inverse results on every call. Throws on invalid parameters,
   * unsupported methods, insufficient data or a failed spline solve. Returns this spectrum.
   */
  calc_background(): this;
  /**
   * Calculate complex chi(R) from chi(k), computing missing normalization and background first.
   * Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048 and kstep inferred
   * from the background grid. The unnormalized forward DFT is multiplied by kstep/sqrt(pi),
   * with no additional 1/N; XrayFFTF explains the formula and units. Clears inverse results and
   * throws on invalid settings or failed prerequisite stages. Returns this spectrum.
   */
  fft(): this;
  /**
   * Calculate real chi(q) by windowing the retained complex Fourier bins in R and performing a
   * conjugate-symmetric inverse transform. Runs missing forward and prerequisite stages first.
   * Forward k-weighting and windowing remain in the output, so this is not generally unweighted
   * chi(k). Throws on inconsistent transform settings or failed prerequisite stages. Returns
   * this spectrum. At least two reported R samples are required even though filtering uses the
   * full internal Fourier bins; rmax_out=0 therefore fails. When reusing a spectrum with a
   * different forward grid, reset previously resolved inverse settings in Next, or create a
   * fresh spectrum in stable 0.2.4.
   */
  ifft(): this;
  /**
   * Clear normalization, background, forward and inverse calculated arrays without discarding
   * the measured inputs, selected E0 or stage settings. User-specified edge-step overrides are
   * retained; a previously estimated step is recomputed by the next normalization. Returns this
   * spectrum. Subsequent getters return undefined until their stages run again. Resolved
   * automatic settings, such as FFT kstep, are retained. Reassign the affected stage settings
   * to request fresh automatic values for changed input grids.
   */
  invalidate_derived(): this;
  /**
   * Return an independent copy of the uniform background k axis in inverse angstroms, beginning
   * at zero and paired with chi(). Its spacing is AUTOBK.kstep (default 0.05). Returns
   * undefined before background removal or after invalidation; this getter never runs a stage.
   */
  k(): Float64Array | undefined;
  /**
   * Return an independent copy of unweighted EXAFS chi(k) = (mu - smooth background) /
   * edge_step, dimensionless and paired with k(). The measured absorption and smooth background
   * are resampled according to the selected background method. Returns undefined before
   * background removal or after invalidation. Forward kweight and window settings do not change
   * this array.
   */
  chi(): Float64Array | undefined;
  /**
   * Return an independent copy of dimensionless normalized absorption, (mu - pre_edge) /
   * edge_step, on the original input energy grid. Here pre_edge is the fitted baseline and
   * edge_step is the selected or fitted absorption jump. Returns undefined before normalization
   * or after invalidation; does not calculate missing results.
   */
  norm(): Float64Array | undefined;
  /**
   * Return an independent copy of dimensionless flattened absorption on the input energy grid.
   * Above E0 this subtracts the fitted post-edge trend from norm(), with an offset that
   * preserves the value at the edge; below E0 it equals norm(). This is a
   * presentation/near-edge quantity, not the background chi(k). Returns undefined before
   * normalization or after invalidation.
   */
  flat(): Float64Array | undefined;
  /**
   * Return an independent copy of the fitted pre-edge baseline, in the same units as input mu
   * and evaluated across the entire original energy grid. The fit uses the selected pre-edge
   * interval, and its extrapolation is subtracted during normalization. Returns undefined
   * before normalization or after invalidation.
   */
  pre_edge(): Float64Array | undefined;
  /**
   * Return an independent copy of the fitted post-edge baseline in input mu units, evaluated on
   * the original energy grid. It includes the pre-edge baseline plus the fitted polynomial for
   * pre-edge-subtracted absorption. It determines the edge step and flattening trend; it is not
   * the AUTOBK background. Returns undefined before normalization or after invalidation.
   */
  post_edge(): Float64Array | undefined;
  /**
   * Return an independent copy of the reported Fourier R axis in angstroms, paired with
   * chir_mag(), chir_real() and chir_imag(). Its spacing is pi/(nfft*kstep), and its extent is
   * limited by rmax_out and the available positive-frequency bins. Peaks are not
   * phase-corrected bond lengths. Returns undefined before fft() or after invalidation.
   */
  r(): Float64Array | undefined;
  /**
   * Return an independent copy of the dimensionless forward window values, paired with
   * kwin_k(). These values exclude the k^kweight factor. The array can have a different
   * length/grid from background k()/chi() with grid=Larch. Returns undefined before fft() or
   * after invalidation.
   */
  kwin(): Float64Array | undefined;
  /**
   * Return an independent copy of the forward window axis in inverse angstroms, paired with
   * kwin(). Input uses the prepared background grid; Larch returns its resampled and possibly
   * extended window grid. This getter does not alter the background k()/chi() arrays. Returns
   * undefined before fft() or after invalidation.
   */
  kwin_k(): Float64Array | undefined;
  /**
   * Return an independent copy of the magnitude sqrt(real^2 + imag^2) of complex chi(R), paired
   * with r(). For dimensionless chi and forward kweight w, units are angstrom^(-(w+1)); at w=2
   * they are inverse cubic angstroms. No peak-height or window-area normalization is applied.
   * Returns undefined before fft() or after invalidation.
   */
  chir_mag(): Float64Array | undefined;
  /**
   * Return an independent copy of the real component of chi(R), paired with r(). Units are
   * angstrom^(-(w+1)), where w is the forward kweight and chi is dimensionless. Uses the
   * negative-exponent forward DFT with amplitude factor kstep/sqrt(pi). Returns undefined
   * before fft() or after invalidation.
   */
  chir_real(): Float64Array | undefined;
  /**
   * Return an independent copy of the imaginary component of chi(R), paired with r(). Units are
   * angstrom^(-(w+1)), where w is the forward kweight and chi is dimensionless. Its sign
   * follows exp(-2*pi*i*j*m/nfft); reversing the Fourier convention changes that sign. Returns
   * undefined before fft() or after invalidation.
   */
  chir_imag(): Float64Array | undefined;
  /**
   * Return an independent copy of the real back-transform axis in inverse angstroms, beginning
   * at zero and paired with chiq(). Its spacing follows the inverse FFT settings and the
   * forward R grid; q distinguishes this possibly filtered/resized grid from background k().
   * Returns undefined before ifft() or after invalidation.
   */
  q(): Float64Array | undefined;
  /**
   * Return an independent copy of the real R-filtered signal, paired with q(). Forward
   * k-weighting and windowing remain, so this is not generally unweighted chi(k). With
   * dimensionless chi, forward kweight w and inverse rweight v, units are angstrom^(v-w);
   * ordinary v=0 retains the units of k^w*chi. Returns undefined before ifft() or after
   * invalidation.
   */
  chiq(): Float64Array | undefined;
}
