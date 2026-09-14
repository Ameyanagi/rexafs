import test from 'node:test';
import assert from 'node:assert/strict';
import { read_measurement } from '../../node.js';

test('shared reader detects units, copies data and validates mappings', () => {
  const doc=read_measurement('energy (keV),mu\n7.1,1\n7.2,2\n');
  try {
    assert.equal(doc.document.format,'text');
    const copy=doc.document;copy.scans[0].columns[0].values[0]=0;
    assert.deepEqual([...doc.arrays().energy],[7100,7200]);
    assert.throws(()=>doc.arrays(-1),/Scan/);
    assert.throws(()=>doc.arrays(2**32),/Scan/);
    assert.throws(()=>doc.arrays(0,{energy_column:99,energy:{kind:'ev'},signal:{kind:'direct',column:1}}),/column/);
  } finally {doc.free();}
  assert.throws(()=>doc.document,/freed/);doc.free();
});

test('arithmetic choices and malformed input match the core',()=>{
  const doc=read_measurement('# energy i0 it if\n7100 10 2 3\n7101 20 4 8\n');
  try {assert.throws(()=>doc.arrays(),/mapping/);const m=doc.document.scans[0].signals[1].mapping;assert.deepEqual([...doc.arrays(0,m).mu],[0.3,0.4]);}finally{doc.free();}
  assert.throws(()=>read_measurement('7100 1\n7101 bad\n'),/line/);
});
