import test from 'node:test';
import assert from 'node:assert/strict';
import { read_measurement } from '../../node.js';
import fs from 'node:fs';
import { sessionFixtures } from './fixtures.mjs';

test('XTUNES projects retain record order and saved tables',()=>{
  const bytes=fs.readFileSync(new URL('xtunes/mixed-raw-analyzed.xtsp', sessionFixtures));
  const doc=read_measurement(bytes);
  try {
    assert.equal(doc.document.format,'xtunes_project');
    assert.equal(doc.document.scans.length,2);
    assert.equal(doc.arrays(0).energy.length,818);
    assert.equal(doc.arrays(1).energy.length,1420);
    assert.equal(doc.arrays(1).mu[0],0.007819);
    assert.ok(doc.document.datasets.some(d=>d.path==='/record_2/Xi Plot' && d.shape[0]===877));
  }finally{doc.free();}
});

test('Wasm reads every valid Larix fixture and rejects all six invalid sessions', () => {
  const root = new URL('larix/', sessionFixtures);
  const manifest = JSON.parse(fs.readFileSync(new URL('manifest.json', root), 'utf8'));
  for (const entry of manifest.files) {
    const bytes = fs.readFileSync(new URL(entry.path, root));
    if (entry.category === 'invalid') { assert.throws(() => read_measurement(bytes), entry.path); continue; }
    const data = read_measurement(bytes);
    try {
      const doc = data.document;
      assert.equal(doc.format, 'larix');
      assert.deepEqual(doc.scans.filter(s => s.signals.length === 1).map(s => s.metadata['larix.symbol']), entry.expected_absorption_groups);
      for (let i = 0; i < doc.scans.length; i++) {
        const scan = doc.scans[i];
        if (scan.signals.length === 1) {
          assert.deepEqual([...data.arrays(i).energy], scan.columns[0].values);
          assert.deepEqual([...data.arrays(i).mu], scan.columns[1].values);
        }
      }
    } finally { data.free(); }
  }
});

test('Larix snapshots preserve complex components, integer bytes and inert metadata', () => {
  const root = new URL('larix/fixtures/valid/', sessionFixtures);
  let data = read_measurement(fs.readFileSync(new URL('two-analyzed.larix', root)));
  try {
    assert.deepEqual([data.arrays(0).energy.length, data.arrays(1).energy.length], [818, 1420]);
    const chir = data.document.datasets.find(d => d.path.endsWith('/chir'));
    assert.deepEqual(chir.shape, [326]);
    const bytes = Buffer.from(chir.attributes['larix.bytes_base64'], 'base64');
    for (let i = 0; i < 326; i++) {
      assert.equal(chir.values[i], bytes.readDoubleLE(i*16));
      assert.equal(chir.imaginary[i], bytes.readDoubleLE(i*16+8));
    }
    assert.throws(() => data.select_datasets([chir.path.replace('/chir', '/r'), chir.path]), /complex/);
  } finally { data.free(); }
  data = read_measurement(fs.readFileSync(new URL('typed-metadata.larix', root)));
  try {
    const doc = data.document;
    const counts = doc.datasets.find(d => d.path.endsWith('/metadata/counts'));
    assert.equal(Buffer.from(counts.attributes['larix.bytes_base64'], 'base64').readBigInt64LE(8), 9007199254740993n);
    assert.equal(doc.scans[0].label, 'Fe foil — 日本語 μ(E).dat');
    assert.match(doc.metadata['larix.session_text'], /2 \* amplitude/);
    assert.match(doc.metadata['larix.session_text'], /preserve repeated keys/);
    doc.metadata['larix.session_text'] = 'changed copy';
    assert.match(data.document.metadata['larix.session_text'], /^##LARIX:/);
  } finally { data.free(); }
});
