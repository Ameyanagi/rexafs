/** Browser workspace limits, independent of the scientific engine's defaults. */
export const SCATTERING_LIMITS = {
  inputBytes: 1024 * 1024,
  totalInputBytes: 8 * 1024 * 1024,
  auxiliaryFiles: 64,
  outputBytes: 128 * 1024 * 1024,
  outputFiles: 10_000,
  plotRows: 100_000,
  logCharacters: 32 * 1024,
  receivedLogBytes: 2 * 1024 * 1024,
  runtimeMilliseconds: 5 * 60 * 1000,
} as const;

/** Validate names in the adapter's relative, case-sensitive virtual filesystem. */
export function validateWorkspacePath(name: string): void {
  if (/[\\\0]/.test(name) || name.split('/').some(part => !part || part === '.' || part === '..')) {
    throw new Error(`Invalid workspace filename: ${name}`);
  }
}

/**
 * Read the first two columns of ReFEFF chi.dat: k in Å⁻¹ and dimensionless χ(k).
 * The upstream codec also writes magnitude, phase and optional diagnostic
 * columns. Reject malformed rows instead of silently dropping data.
 * Source: ReFEFF v0.4.0, crates/refeff-io/src/chi_dat.rs.
 */
export function parseScatteringChi(bytes: Uint8Array): { x: Float64Array; y: Float64Array } {
  if (bytes.byteLength > SCATTERING_LIMITS.outputBytes) throw new Error('chi.dat exceeds the browser output limit.');
  const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  const xs: number[] = [], ys: number[] = [];
  let width: number | undefined;
  for (const [index, line] of text.split(/\r?\n/).entries()) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const fields = trimmed.split(/\s+/);
    if (![4, 5, 6].includes(fields.length) || (width !== undefined && width !== fields.length)) {
      throw new Error(`chi.dat line ${index + 1} has an unexpected column count.`);
    }
    width = fields.length;
    const numeric = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eEdD][+-]?\d+)?$/;
    if (fields.some(value => !numeric.test(value))) throw new Error(`chi.dat line ${index + 1} contains an invalid number.`);
    const values = fields.map(value => Number(value.replace(/[dD]/, 'e')));
    if (values.some(value => !Number.isFinite(value))) throw new Error(`chi.dat line ${index + 1} contains a nonfinite or invalid number.`);
    const [k, chi] = values;
    if (xs.length && k <= xs[xs.length - 1]) throw new Error(`chi.dat line ${index + 1} does not increase in k.`);
    xs.push(k); ys.push(chi);
    if (xs.length > SCATTERING_LIMITS.plotRows) throw new Error('chi.dat exceeds the 100,000-row plot limit. Download the original file to inspect it.');
  }
  if (xs.length < 2) throw new Error('chi.dat needs at least two numeric rows to draw a spectrum.');
  return { x: Float64Array.from(xs), y: Float64Array.from(ys) };
}
