/** Node loads its packaged Wasm automatically; init is a no-op. */
export default function init(): Promise<void>;
export { Spectrum, PrePostEdge, AUTOBK, XrayFFTF, XrayFFTR, NormalizationMethod, BackgroundMethod } from "./types.js";

export type { FFTGrid, FTWindow, AUTOBKSolver, AUTOBKClampScalePolicy, PrePostEdgeOptions, AUTOBKOptions, XrayFFTFOptions, XrayFFTROptions } from "./types.js";
