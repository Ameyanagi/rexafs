import { bindConfiguration } from "./configuration.js";
import initialize, * as core from "./dist/web/rexafs_wasm.js";
import { bindSpectrum } from "./spectrum.js";
let ready = false;
export default async function init(wasm) {
  await initialize(wasm === undefined ? undefined : { module_or_path: wasm });
  ready = true;
}
export const Spectrum = bindSpectrum(core, () => ready);
export const PrePostEdge = bindConfiguration(core.PrePostEdge, ["pre_edge_start", "pre_edge_end", "norm_start", "norm_end", "norm_polyorder", "n_victoreen", "e0", "edge_step"], () => ready);
export const AUTOBK = bindConfiguration(core.AUTOBK, ["ek0", "rbkg", "nknots", "kmin", "kmax", "kstep", "nclamp", "clamp_lo", "clamp_hi", "clamp_lambda", "nfft", "kweight", "dk", "linear_regularization", "linear_condition_limit", "linear_residual_ratio_limit", "linear_fallback_to_lm", "linear_workspace_cache", "window", "solver", "linear_fallback_solver", "clamp_scale_policy"], () => ready);
export const XrayFFTF = bindConfiguration(core.XrayFFTF, ["grid", "rmax_out", "dk", "dk2", "kmin", "kmax", "kweight", "nfft", "kstep", "window"], () => ready);
export const XrayFFTR = bindConfiguration(core.XrayFFTR, ["qmax_out", "dr", "dr2", "rmin", "rmax", "rweight", "nfft", "kstep", "window"], () => ready);
export const { NormalizationMethod, BackgroundMethod } = core;
