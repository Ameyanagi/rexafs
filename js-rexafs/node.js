import { bindConfiguration } from "./configuration.js";
import core from "./dist/node/rexafs_wasm.js";
import { bindSpectrum } from "./spectrum.js";
export default async function init() {}
export const Spectrum = bindSpectrum(core);
export const PrePostEdge = bindConfiguration(core.PrePostEdge, ["pre_edge_start", "pre_edge_end", "norm_start", "norm_end", "norm_polyorder", "n_victoreen", "e0", "edge_step"]);
export const AUTOBK = bindConfiguration(core.AUTOBK, ["ek0", "rbkg", "nknots", "kmin", "kmax", "kstep", "nclamp", "clamp_lo", "clamp_hi", "clamp_lambda", "nfft", "kweight", "dk", "linear_regularization", "linear_condition_limit", "linear_residual_ratio_limit", "linear_fallback_to_lm", "linear_workspace_cache", "window", "solver", "linear_fallback_solver", "clamp_scale_policy"]);
export const XrayFFTF = bindConfiguration(core.XrayFFTF, ["grid", "rmax_out", "dk", "dk2", "kmin", "kmax", "kweight", "nfft", "kstep", "window"]);
export const XrayFFTR = bindConfiguration(core.XrayFFTR, ["qmax_out", "dr", "dr2", "rmin", "rmax", "rweight", "nfft", "kstep", "window"]);
export const { NormalizationMethod, BackgroundMethod } = core;
