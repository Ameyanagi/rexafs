/** Initialize Wasm. Node loads its local Wasm automatically; init is a no-op there. */
export default function init(wasm?: URL | Request | Response | BufferSource | WebAssembly.Module): Promise<void>;
export { Spectrum, PrePostEdge, AUTOBK, XrayFFTF, XrayFFTR, NormalizationMethod, BackgroundMethod } from "./types.js";

export type { FFTGrid, FTWindow, AUTOBKSolver, AUTOBKClampScalePolicy, PrePostEdgeOptions, AUTOBKOptions, XrayFFTFOptions, XrayFFTROptions } from "./types.js";
