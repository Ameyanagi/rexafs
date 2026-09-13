/** Column positions are zero-based; only eV or keV energy inputs are accepted. */
export interface InputColumns {
  energy: number;
  signal: number;
  /** Incident intensity I0; required when signal is transmitted intensity It. */
  reference?: number;
  quantity: 'mu' | 'transmission';
  energyUnit: 'eV' | 'keV';
}

/** Requested processing settings. Undefined E0 selects automatic edge finding. */
export interface AnalysisSettings {
  e0?: number;
  /** Background cutoff radius in angstroms; starts at 1. */
  rbkg: number;
  /** Forward-transform lower/upper wave number, in inverse angstroms. */
  kmin: number;
  kmax: number;
  /** Power applied to chi(k) before the Fourier transform; starts at 2. */
  kweight: number;
  /** Fourier-window taper width in inverse angstroms; starts at 1. */
  dk: number;
  /** Browser allocation limit: a power of two from 256 through 16384. */
  nfft: number;
  grid: 'Input' | 'Larch';
}

/** One independent calculation. A fresh Spectrum resolves defaults for this input. */
export interface AnalysisRequest {
  id: number;
  type: 'process';
  source: { name: string; text: string };
  columns: InputColumns;
  settings: AnalysisSettings;
  /** Same-origin deployment base, including the trailing slash. */
  baseUrl: string;
}

/** Arrays are copied from WASM and transferred to the UI after the spectrum is freed. */
export interface AnalysisResult {
  sourceName: string;
  energy: Float64Array;
  mu: Float64Array;
  norm: Float64Array;
  flat: Float64Array;
  k: Float64Array;
  chi: Float64Array;
  r: Float64Array;
  chirMag: Float64Array;
  /** Resolved absorption-edge energy, in eV. */
  e0: number;
  provenance: Record<string, unknown>;
}

/** Progress marks stage boundaries, not percentages of numerical completion. */
export type AnalysisResponse =
  | { id: number; type: 'progress'; stage: string }
  | { id: number; type: 'result'; result: AnalysisResult }
  | { id: number; type: 'error'; message: string };
