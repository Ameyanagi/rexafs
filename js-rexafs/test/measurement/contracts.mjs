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

test('names and indices resolve consistently in options and existing mappings', () => {
  const data = read_measurement('mu,IFF2,It,energy (keV),IFF1,I0\n0.5,4,2,7.1,3,10\n0.6,10,4,7.2,8,20\n');
  try {
    const result = data.arrays({energy:'energy', i0:'I0', it:'It'});
    assert.deepEqual([...result.energy], [7100,7200]);
    assert.ok([...result.mu].every(v=>Math.abs(v-Math.log(5))<1e-14));
    assert.deepEqual([...data.arrays({energy:3, mu:'mu'}).mu],[0.5,0.6]);
    assert.deepEqual([...data.arrays({energy:'energy', i0:5, iff:['IFF1','IFF2']}).mu],[0.7,0.9]);
    assert.deepEqual([...data.arrays({energy:3, i0:'I0', iff:'IFF1'}).mu],[0.3,0.4]);
    assert.deepEqual([...data.arrays({energy:3, mu:0, energy_unit:'eV'}).energy],[7.1,7.2]);
    const mapping = {energy_column:'energy', energy:{kind:'kev'}, signal:{kind:'transmission',incident:'I0',transmitted:2}};
    assert.deepEqual(data.arrays(0,mapping),result);
    for (const options of [{energy:'energy',mu:'missing'}, {energy:'energy',mu:'MU'},
      {energy:3,mu:0,it:2}, {energy:3,it:2}, {energy:3,mu:0,i0:5}, {mu:0},
      {energy:3,i0:5,iff:[]}, {energy:3,mu:true}, {energy:3,mu:0,energy_unit:'watts'}, {unknown:0}]) {
      assert.throws(()=>data.arrays(options));
    }
    assert.throws(()=>data.arrays({energy:3,mu:0},mapping), /not both/);
  } finally {data.free();}
  const duplicate = read_measurement('energy (eV),mu,mu\n7100,1,2\n');
  try {
    assert.throws(()=>duplicate.arrays({energy:'energy',mu:'mu'}),/ambiguous/);
    assert.deepEqual([...duplicate.arrays({energy:'energy',mu:2}).mu],[2]);
  } finally {duplicate.free();}
});
