/**
 * Return an already-resolved initialization promise for Node.
 *
 * Importing `rexafs/node` loads its packaged WebAssembly engine synchronously, so no explicit
 * initialization is required before constructing objects. This no-op export lets code shared
 * with browser callers use `await init()` in both places. A missing or invalid packaged Wasm
 * asset fails during module import instead.
 */
export default function init(): Promise<void>;
export { Spectrum, PrePostEdge, AUTOBK, XrayFFTF, XrayFFTR, NormalizationMethod, BackgroundMethod } from "./types.js";

export type { FFTGrid, FTWindow, AUTOBKSolver, AUTOBKClampScalePolicy, PrePostEdgeOptions, AUTOBKOptions, XrayFFTFOptions, XrayFFTROptions } from "./types.js";

export { Measurement, read_measurement } from "./types.js";
export type { MeasurementDocument, MeasurementScan, MeasurementColumn, MeasurementDataset, SpectrumMapping, ColumnSelector, MeasurementOptions, EnergyConversion, SignalConversion, SignalCandidate } from "./types.js";

export type { SpectrumMeasurementOptions, MeasurementResult } from "./types.js";
