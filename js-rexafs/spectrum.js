import { validate } from "./validate.js";

function scalarMeasurement(operation, coordinates, options = {}) {
  if (!options || typeof options !== "object" || Array.isArray(options)) throw new TypeError("options must be an object");
  for (const key of Object.keys(options)) {
    if (!["space", "origin", "kweight", "errors"].includes(key)) throw new TypeError(`Unknown measurement option: ${key}`);
  }
  const operations = { point: "Point", mean: "Mean", integral: "Integral", maximum: "Maximum" };
  if (!Object.hasOwn(operations, operation)) throw new TypeError("operation must be point, mean, integral, or maximum");
  const values = operation === "point" ? [coordinates] : coordinates;
  if (!Array.isArray(values) || values.length !== (operation === "point" ? 1 : 2) || values.some(v => typeof v !== "number" || !Number.isFinite(v))) {
    throw new TypeError("point requires one finite number; regions require [start, end]");
  }
  const space = options.space ?? "norm";
  const spaces = { mu: "Mu", norm: "Norm", flat: "Flat", chi: "Chi", fourier: "Fourier" };
  if (!Object.hasOwn(spaces, space)) throw new TypeError("space must be mu, norm, flat, chi, or fourier");
  const kweight = options.kweight ?? 0;
  if (!Number.isInteger(kweight) || kweight < 0 || kweight > 255 || (space !== "chi" && kweight !== 0)) throw new RangeError("kweight must be an integer from 0 to 255, and applies only to chi");
  const origin = options.origin ?? (["chi", "fourier"].includes(space) ? "absolute" : "e0");
  if (!["absolute", "e0"].includes(origin)) throw new TypeError("origin must be e0 or absolute");
  if (options.errors !== undefined && !(options.errors instanceof Float64Array)) throw new TypeError("errors must be a Float64Array");
  return {
    metric: { [operations[operation]]: operation === "point" ? { x: coordinates } : { start: coordinates[0], end: coordinates[1] } },
    space: space === "chi" ? { Chi: { kweight } } : spaces[space],
    origin: origin === "e0" ? "E0" : "Absolute",
  };
}

/**
 * Create the package's Spectrum facade for one generated Wasm module instance.
 * This internal adapter owns one native spectrum per constructed object; callers
 * release it with free(). Array copying, stage execution and cache invalidation
 * occur in Rust. Public signatures, units and stage behavior live in types.d.ts.
 *
 * The facade adds JavaScript type checks, browser readiness checks, fluent return
 * values and direct configuration arguments. It creates temporary algorithm
 * wrappers for direct settings/defaults and frees only those temporary wrappers.
 * Caller-owned settings and algorithm wrappers are borrowed, never consumed.
 */
export function bindSpectrum(core, ready = () => true) {
  return class Spectrum {
    #inner;
    constructor(energy, mu) {
      if (!ready()) throw new Error("Call await init() before creating a spectrum");
      validate(energy, mu);
      this.#inner = core.Spectrum.from_arrays(energy, mu);
    }
    static from_arrays(energy, mu) { return new this(energy, mu); }
    free() { this.#inner.free(); }
    measure(operation, coordinates, options = {}) {
      const definition = scalarMeasurement(operation, coordinates, options);
      return JSON.parse(this.#inner.measure_json(JSON.stringify(definition), options.errors));
    }
    set_spectrum(energy, mu) { validate(energy, mu); this.#inner.set_spectrum(energy, mu); return this; }
    set_e0(e0) {
      if (typeof e0 !== "number") throw new TypeError("e0 must be a number in eV");
      if (!Number.isFinite(e0)) throw new RangeError("e0 must be finite in eV");
      this.#inner.set_e0(e0);
      return this;
    }
    set_normalization_method(method) {
      const parameters = method instanceof core.PrePostEdge;
      const selected = parameters ? core.NormalizationMethod.PrePostEdge(method) : method ?? core.NormalizationMethod.new_prepostedge();
      try { this.#inner.set_normalization_method(selected); }
      finally { if (parameters || method == null) selected.free(); }
      return this;
    }
    set_background_method(method) {
      const parameters = method instanceof core.AUTOBK;
      const selected = parameters ? core.BackgroundMethod.AUTOBK(method) : method ?? core.BackgroundMethod.new_autobk();
      try { this.#inner.set_background_method(selected); }
      finally { if (parameters || method == null) selected.free(); }
      return this;
    }
    set_ifft(parameters) { this.#inner.set_ifft(parameters); return this; }
    set_fft(parameters) { this.#inner.set_fft(parameters); return this; }
    invalidate_derived() { this.#inner.invalidate_derived(); return this; }
    find_e0() { this.#inner.find_e0(); return this; }
    normalize() { this.#inner.normalize(); return this; }
    calc_background() { this.#inner.calc_background(); return this; }
    fft() { this.#inner.fft(); return this; }
    ifft() { this.#inner.ifft(); return this; }
    e0() { return this.#inner.e0(); }
    k() { return this.#inner.k(); }
    chi() { return this.#inner.chi(); }
    norm() { return this.#inner.norm(); }
    flat() { return this.#inner.flat(); }
    pre_edge() { return this.#inner.pre_edge(); }
    post_edge() { return this.#inner.post_edge(); }
    r() { return this.#inner.r(); }
    kwin() { return this.#inner.kwin(); }
    kwin_k() { return this.#inner.kwin_k(); }
    chir_mag() { return this.#inner.chir_mag(); }
    chir_real() { return this.#inner.chir_real(); }
    chir_imag() { return this.#inner.chir_imag(); }
    q() { return this.#inner.q(); }
    chiq() { return this.#inner.chiq(); }
  };
}
