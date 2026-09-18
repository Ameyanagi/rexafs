// Owned native wavelet results; JavaScript never recalculates scientific values.
const models = new WeakSet();
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
function array(value, name) {
  if (!(value instanceof Float64Array)) throw new TypeError(`${name} must be a Float64Array`);
}
export function waveletDefinition(model) {
  if (!models.has(model)) throw new TypeError("model must be a Wavelet");
  return model.to_json();
}
export function bindWavelet(core, ready = () => true) {
  const token = Symbol("owned wavelet result");
  const checkReady = () => {if (!ready()) throw new Error("Call await init() before using wavelets");};
  class Wavelet {
    #inner;
    constructor(k_range, options = {}, owned, key) {
      checkReady();
      if (key === token) this.#inner = owned;
      else {
        if (!options || typeof options !== "object" || Array.isArray(options)) throw new TypeError("options must be an object");
        for (const name of Object.keys(options)) if (!["kweight", "order", "kstep", "rmax", "rstep", "taper", "radii", "nfft"].includes(name)) throw new TypeError(`Unknown wavelet option: ${name}`);
        const config = {k_range:range(k_range,"k_range"),kweight:options.kweight??2,order:options.order??100,kstep:options.kstep??0.05,rmax:options.rmax??6,rstep:options.rstep??null,nfft:options.nfft??null,radii:null,window:"None"};
        for (const [name, low, high] of [["kweight",0,6],["order",1,4096],["nfft",1,262144]]) {
          if (config[name] !== null && (!Number.isInteger(config[name]) || config[name]<low || config[name]>high)) throw new RangeError(`${name} must be an integer from ${low} through ${high}`);
        }
        for (const name of ["kstep","rmax","rstep"]) if (config[name] !== null && finite(config[name],name)<=0) throw new RangeError(`${name} must be positive`);
        const taper = finite(options.taper??0,"taper");
        if (taper<0) throw new RangeError("taper must be nonnegative");
        if (taper>0) config.window = {Cosine:{width:taper}};
        if (options.radii !== undefined) {
          if (!Array.isArray(options.radii) && !(options.radii instanceof Float64Array)) throw new TypeError("radii must be an array of angstrom coordinates");
          config.radii = Array.from(options.radii, v => finite(v,"radii"));
        }
        this.#inner = new core.Wavelet(JSON.stringify(config));
      }
      models.add(this);
    }
    free() {this.#inner.free();}
    calculate(k,chi) {array(k,"k");array(chi,"chi");return wrap(this.#inner.calculate(k,chi));}
    estimate(k) {array(k,"k");return JSON.parse(this.#inner.estimate_json(k));}
    to_json() {return this.#inner.to_json();}
    static from_json(json) {
      checkReady();if (typeof json !== "string") throw new TypeError("json must be a string");
      return new Wavelet(undefined,undefined,new core.Wavelet(json),token);
    }
  }
  class WaveletMap {
    #inner;
    constructor(inner,key) {
      checkReady();if (key !== token) throw new TypeError("Calculate a wavelet or use WaveletMap.from_json");
      this.#inner = inner;
    }
    free() {this.#inner.free();}
    get shape() {return Array.from(this.#inner.shape());}
    get k() {return this.#inner.k();}
    get r() {return this.#inner.r();}
    get input_k() {return this.#inner.input_k();}
    get input_chi() {return this.#inner.input_chi();}
    get prepared_chi() {return this.#inner.prepared_chi();}
    get window() {return this.#inner.window();}
    get support() {return this.#inner.support();}
    get real() {return this.#inner.real();}
    get imaginary() {return this.#inner.imaginary();}
    get magnitude() {return this.#inner.magnitude();}
    phase(relative_floor=0.01) {return this.#inner.phase(finite(relative_floor,"relative_floor"));}
    slice_at_k(k) {return this.#inner.slice_at_k(finite(k,"k"));}
    slice_at_r(r) {return this.#inner.slice_at_r(finite(r,"r"));}
    integral(k_range,r_range) {const [ka,kb]=range(k_range,"k_range"),[ra,rb]=range(r_range,"r_range");return JSON.parse(this.#inner.integral_json(ka,kb,ra,rb));}
    mean(k_range,r_range) {const [ka,kb]=range(k_range,"k_range"),[ra,rb]=range(r_range,"r_range");return JSON.parse(this.#inner.mean_json(ka,kb,ra,rb));}
    maximum(k_range,r_range) {const [ka,kb]=range(k_range,"k_range"),[ra,rb]=range(r_range,"r_range");return JSON.parse(this.#inner.maximum_json(ka,kb,ra,rb));}
    get definition() {return Wavelet.from_json(this.#inner.definition_json());}
    get preparation() {return JSON.parse(this.#inner.preparation_json());}
    get warnings() {return JSON.parse(this.#inner.warnings_json());}
    to_json() {return this.#inner.to_json();}
    static from_json(json) {checkReady();if (typeof json !== "string") throw new TypeError("json must be a string");return wrap(core.WaveletMap.from_json(json));}
  }
  function wrap(inner) {return new WaveletMap(inner,token);}
  return {Wavelet,WaveletMap,wrap};
}
