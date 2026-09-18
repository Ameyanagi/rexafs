import test from "node:test";
import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import {Spectrum,FluorescenceCorrection,MBack,Wavelet} from "../node.js";
import init,{Spectrum as BrowserSpectrum,FluorescenceCorrection as BrowserCorrection} from "../browser.js";
const reference=JSON.parse(await readFile(new URL("../../crates/rexafs/tests/fixtures/analysis/fluorescence/larch-reference.json",import.meta.url)));
function options(c) {return {line:c.line,angles:c.angles,family:c.family,e0:c.e0,pre_edge:c.pre_edge,post_edge:c.post_edge,degree:c.degree};}
function model(c, extra={}) {return new FluorescenceCorrection(c.formula,c.element,"K",{...options(c),...extra});}
const close=(a,b,tolerance)=>assert.ok(Math.abs(a-b)<tolerance,`${a} versus ${b}`);

test("fluorescence reference cases, independent arrays and pinned replay",()=>{
  for (const c of reference.cases) {
    const energy=Float64Array.from(c.energy),mu=Float64Array.from(c.mu),definition=model(c),settings=definition.to_json();
    const result=definition.apply(energy,mu),expected=result.corrected_mu.slice(),saved=result.to_json();
    result.corrected_mu.forEach((v,i)=>close(v,c.corrected_mu[i],reference.tolerances.curve_absolute));
    result.internal.norm.forEach((v,i)=>close(v,c.internal_norm[i],reference.tolerances.curve_absolute));
    close(result.alpha/c.alpha,1,reference.tolerances.atomic_relative);
    assert.equal(definition.to_json(),settings);
    mu.fill(999);result.corrected_mu.fill(-999);result.internal.norm.fill(999);result.atomic.edge=null;
    assert.equal(result.to_json(),saved);assert.deepEqual(Array.from(result.original_mu),c.mu);
    const pinned=result.definition,replay=pinned.apply(energy,result.original_mu);
    assert.deepEqual(replay.corrected_mu,expected);
    const restored=FluorescenceCorrection.from_json(pinned.to_json());
    assert.deepEqual(restored.apply(energy,result.original_mu).factor,replay.factor);
    for(const object of [definition,pinned,restored]) object.free();
    assert.deepEqual(replay.corrected_mu,expected); // Result is ordinary owned JavaScript data.
  }
});

test("corrected Spectrum preserves history and restrictions across normalization and edits",()=>{
  const c=reference.cases[0],energy=Float64Array.from(c.energy),mu=Float64Array.from(c.mu),definition=model(c),source=new Spectrum(energy,mu);
  const corrected=source.correct_fluorescence(definition),record=corrected.fluorescence_correction(),saved=record.to_json();
  assert.equal(source.fluorescence_correction(),undefined);assert.equal(source.norm(),undefined);
  assert.equal(source.absorption_mode(),"unknown");assert.equal(corrected.absorption_mode(),"fluorescence");
  assert.equal(record.input_mode,"unknown");assert.ok(record.warnings.some(v=>v.includes("unknown")));
  corrected.normalize();assert.ok(corrected.norm());
  const mback=new MBack(c.element,"K",{e0:c.e0,pre_edge:c.pre_edge,post_edge:c.post_edge});
  corrected.set_normalization_method(mback).normalize();assert.ok(corrected.mback_result());
  assert.equal(corrected.fluorescence_correction().to_json(),saved);
  corrected.set_spectrum(energy,mu.map(v=>v*1.01));assert.equal(corrected.set_absorption_mode("unknown"),corrected);
  const wavelet=new Wavelet([2,10]);
  for (const run of [()=>corrected.correct_fluorescence(definition),()=>corrected.calc_background(),()=>corrected.fft(),()=>corrected.ifft(),()=>corrected.wavelet(wavelet)]) assert.throws(run);
  assert.equal(corrected.fluorescence_correction().to_json(),saved);
  source.set_absorption_mode("transmission");assert.throws(()=>source.correct_fluorescence(definition),/transmission/);
  assert.throws(()=>source.set_absorption_mode("guess"));
  source.free();definition.free();assert.equal(corrected.fluorescence_correction().to_json(),saved);
  for(const object of [corrected,mback,wavelet]) object.free();
});

test("browser correction ownership and explicit geometry, coverage and singular failures",async()=>{
  const c=reference.cases[0],energy=Float64Array.from(c.energy),mu=Float64Array.from(c.mu);
  assert.throws(()=>new BrowserCorrection(c.formula,c.element,"K",options(c)),/init/);
  await init(await readFile(new URL("../dist/web/rexafs_wasm_bg.wasm",import.meta.url)));
  const definition=new BrowserCorrection(c.formula,c.element,"K",options(c)),source=new BrowserSpectrum(energy,mu);
  const corrected=source.correct_fluorescence(definition);source.free();definition.free();
  assert.ok(corrected.normalize().norm());assert.throws(()=>corrected.fft());corrected.free();
  assert.throws(()=>model(c,{angles:[0,45]}));assert.throws(()=>model(c,{line:undefined}));
  assert.throws(()=>model(c,{typo:1}),/Unknown/);
  for(const extra of [{line:"La1"},{pre_edge:[-9999,-30]}]) {
    const invalid=model(c,extra);assert.throws(()=>invalid.apply(energy,mu));invalid.free();
  }
  const singular=model(c),bad=mu.slice();bad[Math.floor(bad.length/2)]=1e12;
  assert.throws(()=>singular.apply(energy,bad));singular.free();
});
