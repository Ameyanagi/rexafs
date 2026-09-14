/** Bind the shared Rust reader; all parsing and detector arithmetic stay in Rust. */
export function bindMeasurement(core, ready = () => true) {
  return class Measurement {
    #inner;
    constructor(data) {
      if (!ready()) throw new Error("Call await init() before reading a measurement");
      if (typeof data === "string") data = new TextEncoder().encode(data);
      if (!(data instanceof Uint8Array)) throw new TypeError("Measurement data must be a string or Uint8Array");
      this.#inner = new core.Measurement(data);
    }
    #native() {
      if (!this.#inner) throw new Error("Measurement has been freed");
      return this.#inner;
    }
    get document() { return JSON.parse(this.#native().document_json()); }
    arrays(scan = 0, mapping) {
      if (!Number.isSafeInteger(scan) || scan < 0 || scan > 0xffff_ffff) throw new RangeError("Scan must be an integer from 0 through 4294967295");
      const [energy, mu] = JSON.parse(this.#native().arrays_json(scan, mapping === undefined ? undefined : JSON.stringify(mapping)));
      return { energy: Float64Array.from(energy), mu: Float64Array.from(mu) };
    }
    select_datasets(paths) {
      if (!Array.isArray(paths) || paths.some(p => typeof p !== "string")) throw new TypeError("Dataset paths must be an array of strings");
      return this.#native().select_datasets(JSON.stringify(paths));
    }
    free() { this.#inner?.free(); this.#inner = undefined; }
  };
}
