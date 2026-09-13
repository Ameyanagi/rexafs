import type { AnalysisSettings, InputColumns } from './protocol';

/** Application import limits, separate from the underlying Rust API's contract. */
export const MAX_SOURCE_BYTES = 8 * 1024 * 1024;
export const MAX_ROWS = 100_000;

// Match the unchanged default AUTOBK grid in crates/rexafs/src/xafs/background.rs.
const AUTOBK_KSTEP = 0.05;
const AUTOBK_NFFT = 2048;
// Browser budget for each possible dense spline basis: 2 million f64 values
// occupy 16 MB. This bounds that allocation, not the process's total memory.
const MAX_SPLINE_BASIS_VALUES = 2_000_000;
// E - E0 = KTOE * k², in eV and Å⁻¹; mirror xafs/constants.rs (CODATA 2022).
const KTOE = 1e20 * (6.62607015e-34 / (2 * Math.PI)) ** 2
  / (2 * 9.1093837139e-31 * 1.602176634e-19);

/** Split numeric comma/whitespace columns. Blank lines and # comments are ignored. */
function sourceRows(text: string): { line: number; cells: string[] }[] {
  if (new TextEncoder().encode(text).byteLength > MAX_SOURCE_BYTES) {
    throw new Error('This browser preview accepts files up to 8 MiB.');
  }
  const rows: { line: number; cells: string[] }[] = [];
  for (const [index, raw] of text.split(/\r?\n/).entries()) {
    const line = raw.split('#', 1)[0].trim();
    if (!line) continue;
    const cells = line.includes(',') ? line.split(',').map(value => value.trim()) : line.split(/\s+/);
    if (cells.length > 64) throw new Error('This browser preview accepts up to 64 columns.');
    if (rows.length && cells.length !== rows[0].cells.length) {
      throw new Error(`Line ${index + 1} has ${cells.length} columns; expected ${rows[0].cells.length}. Use a consistent numeric table.`);
    }
    rows.push({ line: index + 1, cells });
    if (rows.length > MAX_ROWS) throw new Error('This browser preview accepts up to 100,000 data rows.');
  }
  if (rows.length < 2) throw new Error('Provide at least two numeric data rows; headers and comments must start with #.');
  return rows;
}

/** Inspect columns without sorting or modifying the file; final validation happens in the Worker. */
export function inspectSource(text: string): { columnCount: number; rowCount: number } {
  const rows = sourceRows(text);
  if (rows[0].cells.some(value => !value || !Number.isFinite(Number(value)))) {
    throw new Error(`Line ${rows[0].line} is not numeric. Prefix header lines with #.`);
  }
  return { columnCount: rows[0].cells.length, rowCount: rows.length };
}

/**
 * Read selected columns into owned arrays, converting keV to eV and transmission
 * to mu = ln(I0 / It). Intensities must be positive and in matching units.
 * Reject nonfinite/blank values and non-increasing energy; never silently sort,
 * drop rows or merge repeated energies. No filesystem or network access occurs.
 */
export function parseSource(text: string, columns: InputColumns): { energy: Float64Array; mu: Float64Array } {
  const rows = sourceRows(text);
  if (!['mu', 'transmission'].includes(columns.quantity) || !['eV', 'keV'].includes(columns.energyUnit)) {
    throw new Error('Choose absorption or transmission and an energy unit of eV or keV.');
  }
  const selected = [columns.energy, columns.signal];
  if (columns.quantity === 'transmission') selected.push(columns.reference as number);
  if (selected.some(column => !Number.isInteger(column) || column < 0 || column >= rows[0].cells.length)) {
    throw new Error('Select valid energy and signal columns, plus I0 for transmission.');
  }
  if (new Set(selected).size !== selected.length) throw new Error('Energy and signal roles must use different columns.');
  const energy = new Float64Array(rows.length);
  const mu = new Float64Array(rows.length);
  const numberAt = (row: typeof rows[number], column: number) => {
    const cell = row.cells[column];
    const value = Number(cell);
    if (!cell || !Number.isFinite(value)) throw new Error(`Line ${row.line}, column ${column + 1}: expected a finite number.`);
    return value;
  };
  for (const [index, row] of rows.entries()) {
    energy[index] = numberAt(row, columns.energy) * (columns.energyUnit === 'keV' ? 1000 : 1);
    if (energy[index] <= 0 || energy[index] > 1_000_000) throw new Error(`Line ${row.line}: this preview supports energies above 0 and up to 1,000,000 eV.`);
    if (!Number.isFinite(energy[index]) || (index > 0 && energy[index] <= energy[index - 1])) {
      throw new Error(`Line ${row.line}: energy must be finite and strictly increasing, without duplicates.`);
    }
    const signal = numberAt(row, columns.signal);
    if (columns.quantity === 'transmission') {
      const reference = numberAt(row, columns.reference!);
      if (signal <= 0 || reference <= 0) throw new Error(`Line ${row.line}: I0 and It must both be positive.`);
      mu[index] = Math.log(reference / signal);
    } else mu[index] = signal;
    if (!Number.isFinite(mu[index])) throw new Error(`Line ${row.line}: absorption is not finite; check the intensity ratio.`);
  }
  return { energy, mu };
}

/** Validate this preview's settings before allocating numerical workspaces. */
export function validateSettings(settings: AnalysisSettings): void {
  for (const key of ['rbkg', 'kmin', 'kmax', 'kweight', 'dk', 'nfft'] as const) {
    if (!Number.isFinite(settings[key])) throw new Error(`${key} must be a finite number.`);
  }
  if (settings.e0 !== undefined && !Number.isFinite(settings.e0)) throw new Error('E0 must be finite or blank for Auto.');
  if (settings.rbkg <= 0 || settings.rbkg > 10) throw new Error('Choose Rbkg above 0 and up to 10 angstroms.');
  if (settings.kmin < 0 || settings.kmax <= settings.kmin || settings.kmax > 100) throw new Error('Choose a k range with 0 ≤ kmin < kmax ≤ 100 inverse angstroms.');
  if (settings.dk < 0 || settings.dk > 100) throw new Error('Choose dk from 0 through 100 inverse angstroms.');
  if (!Number.isInteger(settings.kweight) || settings.kweight < 0 || settings.kweight > 3) {
    throw new Error('This browser preview supports k weights 0, 1, 2 and 3.');
  }
  if (!Number.isInteger(settings.nfft) || settings.nfft < 256 || settings.nfft > 16384 || (settings.nfft & (settings.nfft - 1)) !== 0) {
    throw new Error('Choose an FFT length that is a power of two from 256 through 16384.');
  }
  if (!['Input', 'Larch'].includes(settings.grid)) throw new Error('Choose the Input or Larch Fourier grid.');
}

/**
 * Bound this preview's numerical work after normalization has resolved E0.
 * energy is the validated, increasing eV array; settings already passed
 * validateSettings. This mirrors AUTOBK's default k grid and automatic knot
 * count, without changing the core algorithm or silently cropping input.
 *
 * Both Fourier grid choices must fit the complete calculated grid and upper
 * window extent. The core Input mode permits truncation; this preview rejects
 * it because a shorter FFT would otherwise silently omit requested data.
 * The spline estimate includes the raw sample at or immediately before E0,
 * as in background.rs, and bounds both raw and resampled dense basis matrices.
 */
export function validateNumericalWorkspace(energy: Float64Array, e0: number, settings: AnalysisSettings): void {
  if (!Number.isFinite(e0) || energy.length < 2 || e0 <= energy[0] || e0 >= energy[energy.length - 1]) {
    throw new Error('The resolved E0 must lie inside the input energy range.');
  }
  const kmax = Math.sqrt((energy[energy.length - 1] - e0) / KTOE);
  const points = Math.floor(1.01 + kmax / AUTOBK_KSTEP);
  if (points > AUTOBK_NFFT) {
    throw new Error('This browser preview requires the post-edge range to fit AUTOBK’s 2048-point grid (0.05 Å⁻¹ spacing). Import a narrower energy range.');
  }
  const windowPoints = Math.floor(1.01 + (settings.kmax + settings.dk) / AUTOBK_KSTEP);
  const requiredPoints = Math.max(points, windowPoints);
  if (requiredPoints > settings.nfft) {
    throw new Error(`Choose at least ${requiredPoints} FFT points to cover the calculated k grid and window without truncation; select the next available larger FFT length.`);
  }
  let edgeIndex = 0;
  while (edgeIndex + 1 < energy.length && energy[edgeIndex + 1] <= e0) edgeIndex++;
  const rawPoints = energy.length - edgeIndex;
  const knots = Math.min(128, Math.max(5, 1 + Math.floor(2 * settings.rbkg * kmax / Math.PI)));
  if (Math.max(rawPoints, points) * knots > MAX_SPLINE_BASIS_VALUES) {
    throw new Error('This input exceeds the browser preview’s spline-workspace budget (2 million values). Use a smaller dataset or a lower Rbkg.');
  }
}
