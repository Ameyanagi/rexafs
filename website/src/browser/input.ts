import type { AnalysisSettings } from './protocol';

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

/** Validate selected Rust-reader output against the browser processing budget.
 * Preserve acquisition order: sorting or removing duplicates requires a separate
 * user decision before import. Detector arithmetic is owned by the Rust reader.
 */
export function validateImportedArrays(energy: Float64Array, mu: Float64Array): void {
  if (energy.length < 2 || energy.length > MAX_ROWS || energy.length !== mu.length) {
    throw new Error('Select paired arrays with 2 through 100,000 data rows.');
  }
  for (let i = 0; i < energy.length; i++) {
    if (!Number.isFinite(energy[i]) || energy[i] <= 0 || energy[i] > 1_000_000 || !Number.isFinite(mu[i])) {
      throw new Error(`Row ${i + 1}: select finite absorption and energy above 0 and up to 1,000,000 eV.`);
    }
    if (i && energy[i] <= energy[i - 1]) throw new Error(`Row ${i + 1}: energy must be strictly increasing, without duplicates. Review acquisition order before processing.`);
  }
}
