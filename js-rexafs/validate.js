/**
 * Check the JavaScript array types accepted by the Spectrum facade.
 * This internal guard throws TypeError without mutating inputs. It does not check
 * lengths, finite values or energy ordering; the checked Rust constructor performs
 * those validations after the arrays are copied into Wasm. Energy values are in eV.
 */
export function validate(energy, mu) {
  if (!(energy instanceof Float64Array) || !(mu instanceof Float64Array)) {
    throw new TypeError("energy and mu must be Float64Array instances");
  }
}
