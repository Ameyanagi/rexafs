/**
 * Build the package's named-options facade over a generated Wasm settings class.
 * This adapter is internal; consumers import the resulting classes from rexafs.
 *
 * Rust supplies defaults and numerical validation. Only own enumerable string
 * keys are applied, through the same setters used by later property assignment.
 * Unknown keys, arrays, null and non-object options are rejected before allocation.
 * A setter failure releases the newly allocated object before propagating the
 * exception. The ready callback prevents construction before browser init().
 *
 * Options constructor signatures and field semantics are documented in types.d.ts.
 */
export function bindConfiguration(Core, fields, ready = () => true) {
  const allowed = new Set(fields);
  return class extends Core {
    constructor(options = {}) {
      if (!ready()) throw new Error("Call await init() before creating settings");
      if (options === null || typeof options !== "object" || Array.isArray(options)) {
        throw new TypeError("Configuration options must be an object");
      }
      for (const key of Object.keys(options)) {
        if (!allowed.has(key)) throw new TypeError(`Unknown configuration option: ${key}`);
      }
      super();
      try {
        for (const key of Object.keys(options)) this[key] = options[key];
      } catch (error) {
        this.free();
        throw error;
      }
    }
  };
}
