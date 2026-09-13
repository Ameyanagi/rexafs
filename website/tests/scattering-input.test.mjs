import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseScatteringChi, validateWorkspacePath } from '../src/browser/scattering-input.ts';

const bytes = text => new TextEncoder().encode(text);

test('the scattering plot uses k and chi without weighting or normalization', () => {
  const native = readFileSync(new URL('./fixtures/refeff-0.4.0/chi.dat', import.meta.url));
  const expected = native.toString('utf8').split(/\r?\n/)
    .filter(line => line.trim() && !line.trimStart().startsWith('#'))
    .map(line => line.trim().split(/\s+/).map(Number));
  const actual = parseScatteringChi(native);
  assert.equal(actual.x.length, 401);
  assert.deepEqual([...actual.x], expected.map(row => row[0]));
  assert.deepEqual([...actual.y], expected.map(row => row[1]));
  const fortran = parseScatteringChi(bytes('# k chi magnitude phase\n0 -2D-1 8 4\n1d0 3D-1 9 5\n'));
  assert.deepEqual([...fortran.x], [0, 1]);
  assert.deepEqual([...fortran.y], [-0.2, 0.3]);
});

test('malformed spectra fail instead of plotting silently filtered or reordered rows', () => {
  for (const input of [
    '# No spectrum\n', '0 1 2 3\n', '0 1 2 3\n0 2 3 4\n', '1 1 2 3\n0 2 3 4\n',
    '0 1 2 3\n1 NaN 3 4\n', '0 1 2 3\n1 2 Infinity 4\n',
    '0 1 2 3\n1 2 3\n', '0 1 2 3\n1 2 3 4 5\n',
  ]) assert.throws(() => parseScatteringChi(bytes(input)));
  assert.throws(() => parseScatteringChi(Uint8Array.of(0xff, 0xfe)), /encoded data|encoding/i);
  assert.throws(() => parseScatteringChi(bytes(Array.from({ length: 100_001 }, (_, i) => `${i} 1 2 3`).join('\n'))), /100,000-row/);
});

test('virtual file names preserve Unicode and reject paths outside the calculation workspace', () => {
  for (const name of ['model.inp', 'inputs/銅.inp', 'data/feff.dym']) validateWorkspacePath(name);
  for (const name of ['', '/absolute.inp', '../escape.inp', 'a/../b', './model.inp', 'a//b', 'a\\b', 'a\0b', 'a/']) {
    assert.throws(() => validateWorkspacePath(name), /Invalid workspace filename/);
  }
});
