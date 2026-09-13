import test from 'node:test';
import assert from 'node:assert/strict';
import { inspectSource, parseSource, validateSettings, validateNumericalWorkspace, MAX_ROWS } from '../src/browser/input.ts';
const absorption = { energy: 0, signal: 1, quantity: 'mu', energyUnit: 'eV' };
const settings = { rbkg: 1, kmin: 2, kmax: 15, kweight: 2, dk: 1, nfft: 2048, grid: 'Input' };

test('browser import preserves rows and selected units, with CSV and # comments', () => {
  const input = '# energy, mu, extra\n8.97, 0.5, 8\n8.98, 0.6, 9 # note\n';
  assert.deepEqual(inspectSource(input), { columnCount: 3, rowCount: 2 });
  const result = parseSource(input, { ...absorption, energyUnit: 'keV' });
  assert.deepEqual([...result.energy], [8970, 8980]);
  assert.deepEqual([...result.mu], [0.5, 0.6]);
  const transmission = parseSource('8970 10 5\n8980 8 2', { ...absorption, quantity: 'transmission', reference: 1, signal: 2 });
  assert.deepEqual([...transmission.mu], [Math.log(2), Math.log(4)]);
});

test('browser import rejects ambiguous or nonfinite data instead of changing it silently', () => {
  for (const input of ['2 3\n1 4', '1 3\n1 4', '1 3\n2 NaN', '1,3\n2,', '1 3\n2 4 5', '1 3\n1e308 4']) {
    assert.throws(() => parseSource(input, absorption));
  }
  assert.throws(() => inspectSource('energy mu\n1 3\n2 4'), /header/);
  assert.throws(() => parseSource('1 3\n2 4', { ...absorption, signal: 0 }), /different/);
  assert.throws(() => parseSource('1 0 4\n2 2 4', { ...absorption, quantity: 'transmission', reference: 1, signal: 2 }), /positive/);
  assert.throws(() => parseSource('1 3\n2 4', { ...absorption, signal: 3 }), /valid/);
  assert.throws(() => inspectSource('1 2\n'.repeat(MAX_ROWS + 1)), /100,000/);
  assert.throws(() => inspectSource(('1 '.repeat(65) + '\n').repeat(2)), /64 columns/);
});

test('browser settings validate allocation limits before entering WASM', () => {
  validateSettings(settings);
  validateSettings({ ...settings, e0: 8980, grid: 'Larch', nfft: 16384 });
  for (const change of [{ nfft: 0 }, { nfft: 1023 }, { nfft: 32768 }, { rbkg: 0 }, { rbkg: 11 }, { kmax: 1 }, { kmax: 101 }, { e0: NaN }, { dk: -1 }, { kweight: 4 }, { grid: 'typo' }]) {
    assert.throws(() => validateSettings({ ...settings, ...change }));
  }
});

test('browser workspace rejects FFT truncation and broad post-edge allocations after resolving E0', () => {
  const energy = Float64Array.from([8900, 9000, 9100, 10000]);
  for (const grid of ['Input', 'Larch']) {
    validateNumericalWorkspace(energy, 8980, { ...settings, grid, nfft: 512 });
    assert.throws(() => validateNumericalWorkspace(energy, 8980, { ...settings, grid, nfft: 256 }), /without truncation/);
  }
  // A short measured grid still requires room for the selected upper taper.
  assert.throws(() => validateNumericalWorkspace(Float64Array.from([8970, 8980, 8990]), 8980, { ...settings, nfft: 256 }), /window/);
  // File/row limits alone cannot bound the grid inferred from energy coverage.
  assert.throws(() => validateNumericalWorkspace(Float64Array.from([8900, 9000, 1_000_000]), 8980, settings), /AUTOBK/);
  for (const e0 of [NaN, Infinity, 8900, 10000]) {
    assert.throws(() => validateNumericalWorkspace(energy, e0, settings), /resolved E0/);
  }
});

test('browser spline budget accounts for dense input and the requested Rbkg', () => {
  const energy = Float64Array.from({ length: MAX_ROWS }, (_, i) => 8900 + i * 0.01);
  validateNumericalWorkspace(energy, 8980, settings);
  assert.throws(() => validateNumericalWorkspace(energy, 8980, { ...settings, rbkg: 10 }), /spline-workspace budget/);
});
