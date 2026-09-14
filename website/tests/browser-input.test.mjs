import test from 'node:test';
import assert from 'node:assert/strict';
import { validateImportedArrays, validateSettings, validateNumericalWorkspace, MAX_ROWS } from '../src/browser/input.ts';
import { read_measurement } from '../../js-rexafs/node.js';
const settings = { rbkg: 1, kmin: 2, kmax: 15, kweight: 2, dk: 1, nfft: 2048, grid: 'Input' };
function importArrays(text,mapping) {
  const document=read_measurement(text);
  try {const arrays=document.arrays(0,mapping);validateImportedArrays(arrays.energy,arrays.mu);return arrays;}finally{document.free();}
}
test('browser uses the shared reader for numeric CSV, units and detector arithmetic',()=>{
  const data=importArrays('energy_keV,mu,extra\n8.97,0.5,8\n8.98,0.6,9 # note\n');
  assert.deepEqual([...data.energy],[8970,8980]);assert.deepEqual([...data.mu],[0.5,0.6]);
  const trans=importArrays('# energy i0 it\n8970 10 5\n8980 8 2\n');
  assert.ok(Math.abs(trans.mu[0]-Math.log(2))<1e-14);
  assert.ok(Math.abs(trans.mu[1]-Math.log(4))<1e-14);
});
test('browser validates selected arrays without discarding source rows',()=>{
  for(const body of ['2 3\n1 4','1 3\n1 4','1 3\n2 NaN','1,3\n2,','1 3\n2 4 5','1 3\n1e308 4']) {
    assert.throws(()=>importArrays('# energy mu\n'+body));
  }
  assert.throws(()=>importArrays('# energy i0 it\n1 1 0\n2 1 1'));
  assert.throws(()=>importArrays('# energy i0 it if\n7100 10 2 3\n7101 10 2 4'),/mapping/);
  const energy=Float64Array.from({length:MAX_ROWS+1},(_,i)=>i+1);
  assert.throws(()=>validateImportedArrays(energy,energy),/100,000/);
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
