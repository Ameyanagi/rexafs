// Keep numerical defaults and validation in Rust. This adapter only initializes
// named fields, so constructor options behave exactly like assigning properties.
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
