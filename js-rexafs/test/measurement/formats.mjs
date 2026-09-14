import test from 'node:test';
import assert from 'node:assert/strict';
import { read_measurement, Measurement } from '../../node.js';
import fs from 'node:fs';
import { fixtures } from './fixtures.mjs';

test('new BMM fixture retains reference transmission',()=>{
  const file=fs.readFileSync(new URL('samples/nsls-ii/6-bm-bmm/bmm-standards/Fe-K-IronFoil.xdi',fixtures));
  const doc=new Measurement(file);
  try {const {energy,mu}=doc.arrays();assert.equal(energy.length,393);assert.equal(energy[0],6912.001);assert.equal(mu[0],1.981854);}finally{doc.free();}
});

test('Wasm reads HDF5 detector arrays and requires reduction',()=>{
  const file=fs.readFileSync(new URL('samples/max-iv/balder-unconfirmed/parseq-xas/scan-24212_eiger_streaming.h5',fixtures));
  const doc=read_measurement(file);
  try {assert.equal(doc.document.format,'hdf5');assert.equal(doc.document.scans.length,0);assert.ok(doc.document.datasets.length>0);assert.throws(()=>doc.arrays(),/Scan/);}finally{doc.free();}
});

test('explicit HDF5 paths produce copied channels through the binding',()=>{
  const bytes=fs.readFileSync(new URL('samples/max-iv/balder-unconfirmed/parseq-xas/20230318.h5',fixtures));
  const doc=read_measurement(bytes);
  try {
    assert.throws(()=>doc.select_datasets(['/entry24212/measurement/mono1_energy_acs','/entry24213/measurement/albaem-01_ch1']),/matching/);
    const index=doc.select_datasets(['/entry24212/measurement/mono1_energy_acs','/entry24212/measurement/albaem-01_ch1']);
    const result=doc.arrays(index,{energy_column:0,energy:{kind:'ev'},signal:{kind:'direct',column:1}});
    assert.equal(result.energy.length,601);assert.equal(result.energy[0],16065);
    assert.equal(result.mu[0],2.0128017625796508e-6);
  }finally{doc.free();}
});

test('FDMNES relative energy and Demeter sample selection use shared mappings',()=>{
  let doc=read_measurement(fs.readFileSync(new URL('samples/unspecified/fdmnes/xraylarch/FDMNES_2022_Mo2C_out.dat',fixtures)));
  try {
    assert.equal(doc.document.scans[0].columns[0].values[0],-25);
    assert.deepEqual(doc.document.scans[0].signals[0].mapping.energy,{kind:'offset_ev',offset_ev:20000});
    const {energy,mu}=doc.arrays();
    assert.equal(energy.length,559);assert.equal(energy[0],19975);assert.equal(mu[0],0.00011943264);
  }finally{doc.free();}
  doc=read_measurement(fs.readFileSync(new URL('samples/nsls/x23a2/demeter/re4chan.000',fixtures)));
  try {
    assert.throws(()=>doc.arrays(),/mapping/);
    const signals=doc.document.scans[0].signals;assert.equal(signals.length,4);
    const {energy,mu}=doc.arrays(0,signals[1].mapping);
    assert.equal(energy.length,387);assert.ok(Math.abs(mu[0]-Math.log(14600.25/5182.5))<1e-13);
  }finally{doc.free();}
});

test('legacy Athena nested metadata stays intact through Wasm',()=>{
  const doc=read_measurement(fs.readFileSync(new URL('samples/unspecified/athena-legacy/xraylarch/Br.prj',fixtures)));
  try {assert.equal(doc.document.scans.length,5);assert.equal(doc.arrays().energy.length,545);assert.match(doc.document.scans[0].header,/'name' => \{'name' => \{\}\}/);}finally{doc.free();}
});
