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
      if (typeof scan === "object" && scan !== null && !Array.isArray(scan)) {
        if (mapping !== undefined) throw new TypeError("Use options or a positional mapping, not both");
        const options = scan;
        scan = options.scan ?? 0;
        const known = new Set(["scan", "energy", "energy_unit", "mu", "i0", "it", "iff"]);
        for (const key of Object.keys(options)) {
          if (!known.has(key)) throw new TypeError(`Unknown measurement option: ${key}`);
        }
        const { energy, energy_unit, mu, i0, it, iff } = options;
        if ([energy, energy_unit, mu, i0, it, iff].some(v => v !== undefined)) {
          if (energy === undefined || [mu, it, iff].filter(v => v !== undefined).length !== 1)
            throw new TypeError("Specify energy and exactly one of mu, it, or iff");
          let signal;
          if (mu !== undefined) {
            if (i0 !== undefined) throw new TypeError("A direct mu column cannot be combined with i0");
            signal = { kind: "direct", column: mu };
          } else {
            if (i0 === undefined) throw new TypeError("Transmission and fluorescence require i0");
            signal = it !== undefined ? { kind: "transmission", incident: i0, transmitted: it }
              : { kind: "ratio", incident: i0, detectors: Array.isArray(iff) ? iff : [iff] };
          }
          if (energy_unit !== undefined && energy_unit !== "eV" && energy_unit !== "keV")
            throw new TypeError("energy_unit must be eV or keV");
          mapping = { energy_column: energy, signal,
            energy: energy_unit === undefined ? undefined : { kind: energy_unit.toLowerCase() } };
        }
      }
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
