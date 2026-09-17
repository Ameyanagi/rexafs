// Native immutable model builders. Public units/defaults live in types.d.ts.
const models = new WeakMap();
const spectra = new WeakMap();
export function registerPeakSpectrum(wrapper, native) { spectra.set(wrapper, native); }
export function peakDefinition(model) {
  if (!models.has(model)) throw new TypeError("model must be a PeakFit");
  return model.to_json();
}
export function peakResult(json, model) {
  const binding = models.get(model);
  const raw = JSON.parse(json);
  const result = {
    ...raw,
    parameters: Object.fromEntries(Object.entries(raw.parameters.vars).map(([k, v]) => [k, v.value])),
    parameter_errors: Object.fromEntries(Object.entries(raw.parameters.vars).map(([k, v]) => [k, v.stderr])),
    to_json: () => json,
    fitted_model: () => binding.wrap(binding.core.PeakFit.from_result(json)),
  };
  const initial = JSON.stringify(raw.definition);
  Object.defineProperty(result, "definition", { enumerable: true, get: () => binding.wrap(binding.core.PeakFit.from_json(initial)) });
  return result;
}
function finite(...values) {
  if (values.some(v => typeof v !== "number" || !Number.isFinite(v))) throw new TypeError("Peak arguments must be finite numbers");
}
function interval(range) {
  if (!Array.isArray(range) || range.length !== 2) throw new TypeError("range must be [start, end]");
  finite(...range);
  return range;
}
function options(value, allowed) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError("options must be an object");
  for (const key of Object.keys(value)) if (!allowed.includes(key)) throw new TypeError(`Unknown peak option: ${key}`);
}
function component(config, fields) {
  options(config, fields);
  return fields.map(key => {
    const value = config[key];
    if (typeof value !== "number" || !Number.isFinite(value)) throw new TypeError(`${key} is required and must be a finite number`);
    return value;
  });
}

export function bindPeakFit(core, ready = () => true) {
  const internal = Symbol("owned native peak model");
  return class PeakFit {
    #inner;
    constructor(range, owned, token) {
      if (!ready()) throw new Error("Call await init() before creating a peak model");
      this.#inner = token === internal ? owned : new core.PeakFit(...interval(range));
      models.set(this, { core, wrap: inner => PeakFit.#wrap(inner) });
    }
    static #wrap(inner) { return new PeakFit(undefined, inner, internal); }
    #build(method, ...args) {
      if (["gaussian", "lorentzian", "pseudo_voigt", "voigt", "erf_step", "arctan_step", "parameter", "as_baseline"].includes(method) && typeof args[0] !== "string") throw new TypeError("name must be a string");
      return PeakFit.#wrap(this.#inner[method](...args));
    }
    free() { this.#inner.free(); }
    flat() { return this.#build("flat"); }
    raw_mu() { return this.#build("raw_mu"); }
    absolute() { return this.#build("absolute"); }
    reference(energy_ev) { finite(energy_ev); return this.#build("reference", energy_ev); }
    exclude(range) { return this.#build("exclude", ...interval(range)); }
    gaussian(name, config) { return this.#build("gaussian", name, ...component(config, ["center", "area", "fwhm"])); }
    lorentzian(name, config) { return this.#build("lorentzian", name, ...component(config, ["center", "area", "fwhm"])); }
    pseudo_voigt(name, config) { return this.#build("pseudo_voigt", name, ...component(config, ["center", "area", "fwhm", "fraction"])); }
    voigt(name, config) { return this.#build("voigt", name, ...component(config, ["center", "area", "gaussian_fwhm", "lorentzian_fwhm"])); }
    erf_step(name, config) { return this.#build("erf_step", name, ...component(config, ["center", "height", "scale"])); }
    arctan_step(name, config) { return this.#build("arctan_step", name, ...component(config, ["center", "height", "scale"])); }
    constant_baseline(offset = 0) { finite(offset); return this.#build("constant_baseline", offset); }
    linear_baseline(config = {}) {
      options(config, ["offset", "slope"]);
      const offset = config.offset ?? 0, slope = config.slope ?? 0;
      finite(offset, slope); return this.#build("linear_baseline", offset, slope);
    }
    parameter(name, value, config = {}) {
      options(config, ["vary", "bounds", "expression"]);
      finite(value);
      const vary = config.vary ?? true;
      if (typeof vary !== "boolean") throw new TypeError("vary must be boolean");
      const bounds = config.bounds ?? [null, null];
      if (!Array.isArray(bounds) || bounds.length !== 2) throw new TypeError("bounds must be [minimum, maximum], with null for an unbounded side");
      for (const bound of bounds) if (bound !== null) finite(bound);
      if (config.expression !== undefined && typeof config.expression !== "string") throw new TypeError("expression must be a string");
      return this.#build("parameter", name, value, vary, bounds[0] ?? undefined, bounds[1] ?? undefined, config.expression);
    }
    as_baseline(name) { return this.#build("as_baseline", name); }
    solver(config = {}) {
      options(config, ["max_iterations", "tolerance"]);
      const count = config.max_iterations ?? 200;
      const tolerance = config.tolerance ?? 1e-10;
      if (!Number.isInteger(count) || count <= 0 || count > 0xffffffff) throw new RangeError("max_iterations must be a positive 32-bit integer");
      finite(tolerance);
      if (tolerance <= 0) throw new RangeError("tolerance must be positive");
      return this.#build("solver", count, tolerance);
    }
    evaluate(energy, config = {}) {
      options(config, ["e0"]);
      if (!(energy instanceof Float64Array)) throw new TypeError("energy must be a Float64Array");
      if (config.e0 !== undefined) finite(config.e0);
      return this.#inner.evaluate(energy, config.e0);
    }
    initialize_baseline(spectrum, peak_intervals) {
      const native = spectra.get(spectrum);
      if (!native) throw new TypeError("spectrum must be a Spectrum");
      if (!Array.isArray(peak_intervals)) throw new TypeError("peak_intervals must be an array of ranges");
      peak_intervals.forEach(interval);
      return PeakFit.from_json(native.initialize_peaks_json(this.to_json(), JSON.stringify(peak_intervals)));
    }
    fit_batch(inputs) {
      if (!Array.isArray(inputs) || inputs.some(s => !spectra.has(s))) throw new TypeError("spectra must be an array of Spectrum objects");
      // Reject a freed definition even for an empty batch; each fit validates its model.
      this.to_json();
      return inputs.map((s, index) => {
        try { return { index, result: s.fit_peaks(this), error: null }; }
        catch (e) { return { index, result: null, error: String(e?.message ?? e) }; }
      });
    }
    to_json() { return this.#inner.to_json(); }
    static from_json(json) {
      if (!ready()) throw new Error("Call await init() before creating a peak model");
      if (typeof json !== "string") throw new TypeError("json must be a string");
      return PeakFit.#wrap(core.PeakFit.from_json(json));
    }
  };
}
