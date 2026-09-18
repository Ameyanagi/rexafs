// Explicit scientific assumptions; native Rust calculates and pins the historical result.
import { validate } from "./validate.js";
const models = new WeakSet();
function finite(value, name) {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new TypeError(`${name} must be finite`);
  return value;
}
function pair(value, name) {
  if (!Array.isArray(value) || value.length !== 2) throw new TypeError(`${name} must be a pair`);
  return value.map(v => finite(v, name));
}
export function fluorescenceDefinition(model) {
  if (!models.has(model)) throw new TypeError("model must be a FluorescenceCorrection");
  return model.to_json();
}
export function fluorescenceResult(json, Model) {
  if (json === undefined || json === null) return undefined;
  const [raw, definition] = JSON.parse(json);
  const saved = JSON.stringify(raw), model = JSON.stringify(definition);
  const result = { method:raw.method, input_mode: raw.input_mode.toLowerCase(),
    alpha:raw.alpha, geometry_ratio:raw.geometry_ratio, internal:raw.internal,
    minimum_denominator:raw.minimum_denominator, maximum_amplification:raw.maximum_amplification,
    singularity_threshold:raw.singularity_threshold, warnings:raw.warnings,
    atomic: {edge:raw.edge, emission:raw.emission, attenuation:raw.attenuation},
    to_json: () => saved };
  for (const field of ["energy", "original_mu", "corrected_mu", "factor", "denominator"]) {
    result[field] = Float64Array.from(raw[field]);
  }
  Object.defineProperty(result, "definition", { enumerable: true, get: () => Model.from_json(model) });
  return result;
}
export function bindFluorescence(core, ready = () => true) {
  const token = Symbol("owned correction model");
  return class FluorescenceCorrection {
    #inner;
    constructor(formula, element, edge, options, owned, key) {
      if (!ready()) throw new Error("Call await init() before creating fluorescence settings");
      if (key === token) this.#inner = owned;
      else {
        for (const [name, value] of Object.entries({formula,element,edge})) {
          if (typeof value !== "string" || !value) throw new TypeError(`${name} must be a nonempty string`);
        }
        if (!options || typeof options !== "object" || Array.isArray(options)) throw new TypeError("options must specify line and measured angles");
        for (const key of Object.keys(options)) {
          if (!["line", "angles", "family", "e0", "pre_edge", "post_edge", "degree"].includes(key)) throw new TypeError(`Unknown fluorescence option: ${key}`);
        }
        if (typeof options.line !== "string" || !options.line) throw new TypeError("line must be a nonempty string");
        if (options.family !== undefined && typeof options.family !== "boolean") throw new TypeError("family must be boolean");
        const [incidence_deg, exit_deg] = pair(options.angles, "angles");
        if (incidence_deg <= 0 || incidence_deg > 90 || exit_deg <= 0 || exit_deg > 90) throw new RangeError("surface angles must be greater than 0 and at most 90 degrees");
        const config = {formula,element,edge,emission:{[options.family ? "Family" : "Line"]:options.line},incidence_deg,exit_deg,
          e0:null,pre_edge:null,post_edge:null,degree:options.degree??1,reference:null};
        if (!Number.isInteger(config.degree) || config.degree<0 || config.degree>5) throw new RangeError("degree must be an integer from 0 through 5");
        if (options.e0 !== undefined) config.e0 = finite(options.e0, "e0");
        for (const field of ["pre_edge", "post_edge"]) if (options[field] !== undefined) {
          config[field] = pair(options[field], field);
          if (config[field][0] >= config[field][1]) throw new RangeError(`${field} must be increasing`);
        }
        this.#inner = new core.FluorescenceCorrection(JSON.stringify(config));
      }
      models.add(this);
    }
    free() { this.#inner.free(); }
    apply(energy, mu) {
      validate(energy, mu);
      return fluorescenceResult(this.#inner.apply_json(energy, mu), FluorescenceCorrection);
    }
    to_json() { return this.#inner.to_json(); }
    static from_json(json) {
      if (!ready()) throw new Error("Call await init() before creating fluorescence settings");
      if (typeof json !== "string") throw new TypeError("json must be a string");
      return new FluorescenceCorrection(undefined,undefined,undefined,undefined,new core.FluorescenceCorrection(json),token);
    }
  };
}
