/**
 * Load and initialize the browser WebAssembly engine before constructing spectra, settings or
 * algorithm wrappers. Await the returned promise before calling an API.
 *
 * With no argument, the generated loader fetches the packaged `.wasm` file beside its
 * JavaScript glue module. Supply a URL, Request, Response, byte buffer or compiled
 * WebAssembly.Module when a bundler moves the asset. Fetch, compilation and instantiation
 * failures reject the promise. Calling init again after a successful initialization reuses the
 * initialized engine and ignores a different Wasm argument. Await one initialization before
 * starting processing; init() is not a way to replace an engine already in use.
 *
 * The Node entry point loads its local Wasm automatically; init is a no-op there.
 */
export default function init(wasm?: URL | Request | Response | BufferSource | WebAssembly.Module): Promise<void>;
export { Spectrum, PrePostEdge, AUTOBK, XrayFFTF, XrayFFTR, NormalizationMethod, BackgroundMethod } from "./types.js";

export type { FFTGrid, FTWindow, AUTOBKSolver, AUTOBKClampScalePolicy, PrePostEdgeOptions, AUTOBKOptions, XrayFFTFOptions, XrayFFTROptions } from "./types.js";

export { Measurement, read_measurement } from "./types.js";
export type { MeasurementDocument, MeasurementScan, MeasurementColumn, MeasurementDataset, SpectrumMapping, ColumnSelector, MeasurementOptions, EnergyConversion, SignalConversion, SignalCandidate } from "./types.js";

export type { SpectrumMeasurementOptions, MeasurementResult } from "./types.js";
export { PeakFit } from "./types.js";
export type { PeakFitResult, PeakContribution, PeakFitOutcome, PeakParameterOptions } from "./types.js";

export type { PeakOptions, PseudoVoigtOptions, VoigtOptions, StepOptions } from "./types.js";

export { MBack, MbackErfc } from "./types.js";
export type { MbackOptions, MbackErfcOptions, MbackResult, AtomicReference } from "./types.js";

export { Wavelet, WaveletMap } from "./types.js";
export type { WaveletOptions, WaveletSize, WaveletPreparation, WaveletRegionValue } from "./types.js";
