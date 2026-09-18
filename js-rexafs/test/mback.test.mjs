import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { MBack, MbackErfc, Spectrum } from "../node.js";
import init, { MBack as BrowserMBack, Spectrum as BrowserSpectrum } from "../browser.js";
const reference = JSON.parse(await readFile(new URL("../../crates/rexafs/tests/fixtures/analysis/mback/larch-reference.json", import.meta.url)));
const close = (a,b,delta) => assert.ok(Math.abs(a-b)<delta, `${a} versus ${b}`);
function options(c) {return {e0:c.e0, pre_edge:c.pre_edge, post_edge:c.post_edge, degree:c.degree,
  ...(c.erfc ? {erfc:new MbackErfc("Ka1", {width:[500,1500],amplitude:[0,10]})} : {})};}

test("native MBACK reference, owned arrays, replay and spectrum invalidation", () => {
  for (const c of reference.cases) {
    const model = new MBack("Cu", "K", options(c));
    const energy=Float64Array.from(c.energy),mu=Float64Array.from(c.mu),initial=model.to_json();
    const result=model.fit(energy,mu);
    for (const key of ["norm","fpp","background","pre_curve","post_curve"]) result[key].forEach((v,i)=>close(v,c[key][i],1e-6));
    close(result.objective,c.objective,1e-10);
    assert.equal(model.to_json(),initial);
    assert.equal(result.reference.data.data_version,"9.2");
    const spectrum=new Spectrum(energy,mu).set_normalization_method(model).normalize();
    const saved=spectrum.mback_result(),snapshot=saved.to_json();
    assert.deepEqual(Array.from(spectrum.norm()),saved.norm);
    saved.norm[0]=999;
    assert.notEqual(spectrum.mback_result().norm[0],999);
    assert.equal(saved.to_json(),snapshot);
    const replay=saved.definition;
    assert.ok(replay instanceof MBack);
    const expected=JSON.parse(snapshot);
    assert.deepEqual(replay.fit(energy,mu).norm,expected.norm);
    spectrum.set_e0(c.e0+1);
    assert.equal(spectrum.mback_result(),undefined);
    assert.deepEqual(Array.from(mu),c.mu);
    for(const object of [model,spectrum,replay]) object.free();
  }
});

test("browser readiness, invalid ranges and explicit unavailable historical reference", async () => {
  assert.throws(()=>new BrowserMBack("Cu","K"),/init/);
  await init(await readFile(new URL("../dist/web/rexafs_wasm_bg.wasm",import.meta.url)));
  const c=reference.cases[0],energy=Float64Array.from(c.energy),mu=Float64Array.from(c.mu);
  const model=new BrowserMBack("Cu","K",options(c));
  const spectrum=new BrowserSpectrum(energy,mu).set_normalization_method(model).normalize();
  const result=spectrum.mback_result();
  result.norm.forEach((v,i)=>close(v,c.norm[i],1e-6));
  const replay=result.definition;
  const definition=JSON.parse(replay.to_json());
  definition.options.reference.data.data_sha256="not-available";
  const missing=BrowserMBack.from_json(JSON.stringify(definition));
  assert.throws(()=>missing.fit(energy,mu),/archived atomic reference/);
  const outside=new MBack("Cu","K",{e0:c.e0,pre_edge:[-5000,-50]});
  assert.throws(()=>outside.fit(energy,mu),/outside the scan/);
  assert.throws(()=>new MBack("Cu","K",{pre_edge:[NaN,0]}),/finite/);
  assert.throws(()=>new MBack("Cu","K",{typo:2}),/Unknown MBACK/);
  for(const object of [model,spectrum,replay,missing,outside]) object.free();
});
