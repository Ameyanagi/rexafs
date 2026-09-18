import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { PeakFit, Spectrum, PrePostEdge } from "../node.js";
import initBrowser, { PeakFit as BrowserPeakFit, Spectrum as BrowserSpectrum } from "../browser.js";
const reference = JSON.parse(await readFile(new URL("../../crates/rexafs/tests/fixtures/analysis/xanes-peaks/lmfit-reference.json", import.meta.url)));
function definition(case_, Type = PeakFit) {
  const p = case_.initial;
  const base = new Type([-15, 35]).raw_mu().reference(9000);
  const options = { center: p.p_center, area: p.p_area };
  const method = { Gaussian: "gaussian", Lorentzian: "lorentzian", PseudoVoigt: "pseudo_voigt", Voigt: "voigt" }[case_.shape];
  if (method === "voigt") Object.assign(options, { gaussian_fwhm: p.p_width, lorentzian_fwhm: p.p_lorentz_width });
  else options.fwhm = p.p_width;
  if (method === "pseudo_voigt") options.fraction = p.p_fraction;
  return base[method]("p", options).linear_baseline({ offset: p.baseline_offset, slope: p.baseline_slope }).exclude([9, 10]);
}
function close(actual, expected, tolerance) { assert.ok(Math.abs(actual - expected) < tolerance, `${actual} != ${expected}`); }

test("native profiles, weighted covariance and masks match pinned lmfit", () => {
  for (const case_ of reference.cases) {
    const model = definition(case_);
    const initial = model.to_json();
    const source = new Spectrum(Float64Array.from(case_.energy), Float64Array.from(case_.signal));
    const result = source.fit_peaks(model, { errors: Float64Array.from(case_.sigma) });
    assert.equal(result.termination, "Converged");
    assert.equal(result.uncertainty_unavailable, null);
    assert.deepEqual(result.source_indices, case_.source_indices);
    result.model.forEach((value, i) => close(value, case_.model[i], 2e-7));
    close(result.objective, case_.objective, 1e-6);
    for (const [name, expected] of Object.entries(case_.parameters)) {
      close(result.parameters[name], expected.value, 1e-4);
      close(result.parameter_errors[name] / expected.stderr, 1, 3e-4);
    }
    close(result.components[0].center_ev, 9000 + result.parameters.p_center, 1e-10);
    assert.equal(result.components[1].area, null);
    const initialCopy = result.definition;
    assert.equal(initialCopy.to_json(), initial);
    assert.equal(model.to_json(), initial);
    const fitted = result.fitted_model();
    fitted.evaluate(Float64Array.from(result.energy)).forEach((v, i) => close(v, result.model[i], 1e-12));
    const snapshot = result.to_json();
    result.model.fill(100); result.components[0].curve.fill(100);
    assert.equal(result.to_json(), snapshot);
    assert.equal(source.norm(), undefined);
    assert.equal(source.e0(), undefined);
    for (const owned of [model, source, fitted, initialCopy]) owned.free();
  }
});

test("Norm/Flat automatic preparation and ordered batch failures", () => {
  const energy = Float64Array.from({ length: 1001 }, (_, i) => 8900 + i);
  const signal = energy.map(e => .2 + .0001 * (e - 9000) + 1 / (1 + Math.exp(-(e - 9000) / 2)));
  const settings = new PrePostEdge({ e0: 9000 });
  const source = new Spectrum(energy, signal).set_normalization_method(settings);
  const explicit = new Spectrum(energy, signal).set_normalization_method(settings).normalize();
  const original = new PeakFit([-50, 100]).erf_step("edge", { center: 0, height: 1, scale: 4 }).constant_baseline();
  const flat = original.flat();
  for (const model of [original, flat]) {
    const a = source.fit_peaks(model), b = explicit.fit_peaks(model);
    assert.deepEqual(a.data, b.data); assert.deepEqual(a.model, b.model);
    assert.equal(a.origin_ev, 9000);
  }
  assert.equal(source.norm(), undefined); assert.equal(source.e0(), 9000);
  const short = new Spectrum(new Float64Array([8999, 9000, 9001]), new Float64Array([1, 1, 1]));
  const rows = flat.fit_batch([source, short, source]);
  assert.deepEqual(rows.map(r => r.index), [0, 1, 2]);
  assert.equal(rows[0].error, null); assert.equal(rows[1].result, null); assert.ok(rows[1].error);
  assert.deepEqual(rows[0].result.parameters, rows[2].result.parameters);
  assert.deepEqual(flat.fit_batch([]), []);
  assert.equal(JSON.parse(original.to_json()).space, "Norm");
  for (const owned of [settings, source, explicit, original, flat, short]) owned.free();
});

test("baseline initialization, constraints and clear invalid-option errors", () => {
  const energy = Float64Array.from({ length: 401 }, (_, i) => -20 + i / 8);
  const truth = new PeakFit([-20, 30]).raw_mu().absolute().gaussian("p", { center: 3, area: 4, fwhm: 3 }).linear_baseline({ offset: .2, slope: .001 });
  const source = new Spectrum(energy, truth.evaluate(energy));
  const start = truth.parameter("baseline_offset", 0).parameter("baseline_slope", 0);
  const initialized = start.initialize_baseline(source, [[-5, 12]]);
  close(JSON.parse(initialized.to_json()).parameters.vars.baseline_offset.value, .2, 1e-8);
  const tied = start.parameter("p_width", 3, { vary: false }).parameter("baseline_slope", .001, { expression: "p_width/3000" });
  const fit = source.fit_peaks(tied);
  assert.equal(fit.parameters.p_width, 3); close(fit.parameters.baseline_slope, .001, 1e-12);
  assert.equal(fit.free_parameters, 3);
  assert.throws(() => start.parameter("typo", 1), /Unknown peak parameter/);
  assert.throws(() => start.gaussian("q", { center: 1, area: 1, fwhm: 1, typo: 1 }), /Unknown peak option/);
  assert.throws(() => start.evaluate([1, 2]), /Float64Array/);
  assert.throws(() => source.fit_peaks(start, { errors: new Float64Array(401) }));
  assert.throws(() => source.fit_peaks(start, { error: new Float64Array(401) }), /Unknown peak-fit option/);
  assert.throws(() => source.fit_peaks({}), /PeakFit/);
  assert.throws(() => start.solver({ max_iterations: 0 }));
  for (const owned of [truth, source, start, initialized, tied]) owned.free();
});

test("browser binding initialization and model reuse match Node", async () => {
  assert.throws(() => new BrowserPeakFit([-10, 10]), /init/);
  await initBrowser(await readFile(new URL("../dist/web/rexafs_wasm_bg.wasm", import.meta.url)));
  const case_ = reference.cases[0];
  const source = new BrowserSpectrum(Float64Array.from(case_.energy), Float64Array.from(case_.signal));
  const model = definition(case_, BrowserPeakFit);
  const copy = PeakFit.from_json(model.to_json());
  const a = source.fit_peaks(model), b = source.fit_peaks(copy);
  assert.deepEqual(a.parameters, b.parameters);
  assert.equal(a.termination, "Converged");
  for (const owned of [source, model, copy]) owned.free();
});
