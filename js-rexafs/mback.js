// Named MBACK options with native computation and versioned result ownership.
import { validate } from "./validate.js";
const models = new WeakMap();
function object(value, keys) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError("options must be an object");
  for (const key of Object.keys(value)) if (!keys.includes(key)) throw new TypeError(`Unknown MBACK option: ${key}`);
}
function finite(value, name) {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new TypeError(`${name} must be finite`);
  return value;
}
function range(value, name) {
  if (!Array.isArray(value) || value.length !== 2) throw new TypeError(`${name} must be [start, end]`);
  value.forEach(v => finite(v, name));
  if (value[0] >= value[1]) throw new RangeError(`${name} must be increasing`);
  return [...value];
}
export class MbackErfc {
  constructor(line, options) {
    object(options, ["width", "amplitude", "family"]);
    if (typeof line !== "string" || !line) throw new TypeError("line must be a nonempty string");
    if (options.family !== undefined && typeof options.family !== "boolean") throw new TypeError("family must be boolean");
    this.emission = Object.freeze({ [options.family ? "Family" : "Line"]: line });
    this.width_ev = Object.freeze(range(options.width, "width"));
    this.amplitude = Object.freeze(range(options.amplitude, "amplitude"));
    if (this.width_ev[0] <= 0) throw new RangeError("width must be positive in eV");
    Object.freeze(this);
  }
}
export function isMBack(model) { return models.has(model); }
export function mbackDefinition(model) {
  if (!models.has(model)) throw new TypeError("model must be a MBack");
  return model.to_json();
}
export function mbackResult(json, MBack) {
  if (json === undefined || json === null) return undefined;
  const raw = JSON.parse(json);
  const saved = JSON.stringify({ e0: raw.requested_e0, options: { ...raw.requested, reference: raw.reference } });
  const result = { ...raw, to_json: () => json };
  Object.defineProperty(result, "definition", { enumerable: true, get: () => MBack.from_json(saved) });
  return result;
}
export function bindMBack(core, ready = () => true) {
  const token = Symbol("owned MBACK model");
  return class MBack {
    #inner;
    constructor(element, edge, options = {}, owned, key) {
      if (!ready()) throw new Error("Call await init() before creating MBACK settings");
      if (key === token) this.#inner = owned;
      else {
        if (typeof element !== "string" || !element || typeof edge !== "string" || !edge) throw new TypeError("element and edge must be nonempty strings");
        object(options, ["e0", "pre_edge", "post_edge", "degree", "erfc"]);
        const normalized = {};
        if (options.e0 !== undefined) normalized.e0 = finite(options.e0, "e0");
        for (const name of ["pre_edge", "post_edge"]) if (options[name] !== undefined) normalized[name] = range(options[name], name);
        if (options.degree !== undefined) {
          if (!Number.isInteger(options.degree) || options.degree < 0 || options.degree > 5) throw new RangeError("degree must be an integer from 0 through 5");
          normalized.degree = options.degree;
        }
        if (options.erfc !== undefined) {
          if (!(options.erfc instanceof MbackErfc)) throw new TypeError("erfc must be a MbackErfc");
          normalized.erfc = options.erfc;
        }
        this.#inner = new core.MBack(element, edge, JSON.stringify(normalized));
      }
      models.set(this, true);
    }
    free() { this.#inner.free(); }
    fit(energy, mu) {
      validate(energy, mu);
      return mbackResult(this.#inner.fit_json(energy, mu), MBack);
    }
    to_json() { return this.#inner.to_json(); }
    static from_json(json) {
      if (!ready()) throw new Error("Call await init() before creating MBACK settings");
      if (typeof json !== "string") throw new TypeError("json must be a string");
      return new MBack(undefined, undefined, undefined, core.MBack.from_json(json), token);
    }
  };
}
