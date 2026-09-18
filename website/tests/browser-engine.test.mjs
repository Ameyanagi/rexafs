import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('built website engine loads its complete module graph and reads the Cu example', async () => {
  // Import the deployed assets, not the source package: missing runtime helpers
  // otherwise go unnoticed by tests that use the Node bindings directly.
  const engine = await import('../dist/wasm/browser.js');
  await engine.default(readFileSync(new URL('../dist/wasm/dist/web/rexafs_wasm_bg.wasm', import.meta.url)));
  const source = readFileSync(new URL('../dist/examples/cu_150k.xmu', import.meta.url));
  const rows = source.toString('utf8').split(/\r?\n/)
    .map(line => line.split('#')[0].trim()).filter(Boolean)
    .map(line => line.split(/\s+/).map(Number));
  const measurement = engine.read_measurement(source);
  try {
    const { energy, mu } = measurement.arrays(0);
    assert.deepEqual([...energy], rows.map(row => row[0]));
    assert.deepEqual([...mu], rows.map(row => row[1]));
  } finally {
    measurement.free();
  }
});
