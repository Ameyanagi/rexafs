import { MAX_SOURCE_BYTES, validateImportedArrays, validateSettings, validateNumericalWorkspace } from './input';
import type { AnalysisRequest, AnalysisResponse, AnalysisResult, InspectionRequest, InspectionResponse } from './protocol';

type Engine = typeof import('../../../js-rexafs/index');
interface EngineManifest { version: string; commit: string; dirty: boolean; wasmSha256: string; channel: string }
const port = self as unknown as {
  onmessage: ((event: MessageEvent<AnalysisRequest | InspectionRequest>) => void) | null;
  postMessage(message: AnalysisResponse | InspectionResponse, transfer?: Transferable[]): void;
};
let loaded: Promise<{ engine: Engine; manifest: EngineManifest }> | undefined;

/** Load only same-origin, build-produced assets. A failed fetch can be retried. */
function loadEngine(baseUrl: string) {
  if (!loaded) {
    const base = new URL(baseUrl, self.location.origin);
    if (base.origin !== self.location.origin) throw new Error('The engine must load from this website.');
    loaded = (async () => {
      const moduleUrl = new URL('wasm/browser.js', base).href;
      const engine = await import(/* @vite-ignore */ moduleUrl) as Engine;
      const response = await fetch(new URL('wasm/manifest.json', base));
      if (!response.ok) throw new Error('The browser engine manifest could not be loaded. Please reload the page.');
      const manifest = await response.json() as EngineManifest;
      const binaryResponse = await fetch(new URL('wasm/dist/web/rexafs_wasm_bg.wasm', base));
      if (!binaryResponse.ok) throw new Error('The browser engine could not be downloaded. Please reload the page.');
      const bytes = await binaryResponse.arrayBuffer();
      const actualHash = hex(await crypto.subtle.digest('SHA-256', bytes));
      if (actualHash !== manifest.wasmSha256) throw new Error('The engine and its manifest differ. Reload the page to obtain a consistent build.');
      await engine.default(bytes);
      return { engine, manifest };
    })().catch(error => { loaded = undefined; throw error; });
  }
  return loaded;
}

function hex(buffer: ArrayBuffer): string {
  return Array.from(new Uint8Array(buffer), byte => byte.toString(16).padStart(2, '0')).join('');
}

/** Content identity is exported for reproducibility; source bytes stay in this browser. */
async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new Uint8Array(bytes));
  return hex(digest);
}

/** Each request owns a fresh spectrum; all native allocations are released on success or failure. */
port.onmessage = async ({ data }) => {
  if (!['process','inspect'].includes(data.type)) return;
  const progress = (stage: string) => port.postMessage({ id: data.id, type: 'progress', stage });
  try {
    if (data.source.bytes.byteLength > MAX_SOURCE_BYTES) throw new Error('This browser preview accepts files up to 8 MiB.');
    if (data.type === 'process') progress('Loading engine');
    const { engine, manifest } = await loadEngine(data.baseUrl);
    const measurement = engine.read_measurement(data.source.bytes);
    let energy: Float64Array, mu: Float64Array;
    let sourceFormat: string, sourceWarnings: string[];
    try {
      const paths = data.type === 'inspect' ? data.datasetPaths : data.selection.datasetPaths;
      const selected = paths?.length ? measurement.select_datasets(paths) : undefined;
      const document = measurement.document;
      if (data.type === 'inspect') {
        const rowCounts = document.scans.map(scan => scan.columns[0]?.values.length ?? 0);
        for (const scan of document.scans) {
          for (const column of scan.columns) column.values = column.values.slice(0, 3);
          scan.header = scan.header.slice(0, 32_768);
        }
        // Previews retain shapes and complex markers, with bounded text/values.
        for (const dataset of document.datasets) {
          dataset.values = dataset.values.slice(0, 3);
          if (dataset.imaginary) dataset.imaginary = dataset.imaginary.slice(0, 3);
          for (const key of Object.keys(dataset.attributes)) dataset.attributes[key] = dataset.attributes[key].slice(0, 32768);
        }
        for (const key of Object.keys(document.metadata)) document.metadata[key] = document.metadata[key].slice(0, 32768);
        port.postMessage({id:data.id,type:'inspection',preview:{document,rowCounts}});
        return;
      }
      const scan = selected ?? data.selection.scan;
      ({energy,mu} = measurement.arrays(scan, data.selection.mapping));
      sourceFormat = document.format;
      sourceWarnings = [...document.warnings,...(document.scans[scan]?.warnings ?? [])];
    } finally { measurement.free(); }
    if (data.type !== 'process') return;
    validateSettings(data.settings);
    validateImportedArrays(energy, mu);
    const inputHash = await sha256(data.source.bytes);
    const spectrum = new engine.Spectrum(energy, mu);
    let result: AnalysisResult;
    const started = performance.now();
    try {
      if (data.settings.e0 !== undefined) spectrum.set_e0(data.settings.e0);
      const background = new engine.AUTOBK({ rbkg: data.settings.rbkg });
      try { spectrum.set_background_method(background); } finally { background.free(); }
      const transform = new engine.XrayFFTF({
        kmin: data.settings.kmin, kmax: data.settings.kmax, kweight: data.settings.kweight,
        dk: data.settings.dk, nfft: data.settings.nfft, grid: data.settings.grid,
      });
      try { spectrum.set_fft(transform); } finally { transform.free(); }
      progress('Normalizing');
      spectrum.normalize();
      const resolvedE0 = spectrum.e0();
      if (resolvedE0 === undefined) throw new Error('Normalization did not return an edge energy.');
      validateNumericalWorkspace(energy, resolvedE0, data.settings);
      progress('Removing background');
      spectrum.calc_background();
      progress('Fourier transform');
      spectrum.fft();
      const norm = spectrum.norm(), flat = spectrum.flat(), k = spectrum.k(), chi = spectrum.chi();
      const r = spectrum.r(), chirMag = spectrum.chir_mag(), e0 = spectrum.e0();
      if (!norm || !flat || !k || !chi || !r || !chirMag || e0 === undefined) throw new Error('The engine returned incomplete processing results.');
      for (const array of [norm, flat, k, chi, r, chirMag]) {
        if (!array.every(Number.isFinite)) throw new Error('Processing produced nonfinite results. Review the input and settings.');
      }
      result = {
        sourceName: data.source.name, energy, mu, norm, flat, k, chi, r, chirMag, e0,
        provenance: {
          schema: 'rexafs-browser-analysis-v2', createdAt: new Date().toISOString(),
          engine: manifest, input: { name: data.source.name, bytesSha256: inputHash, format: sourceFormat, warnings: sourceWarnings, rows: energy.length, selection: data.selection },
          requested: data.settings, resolved: { e0 },
          defaults: { normalization: 'PrePostEdge automatic', background: 'AUTOBK defaults with requested rbkg', fourierWindow: 'KaiserBessel' },
          units: { energy: 'eV', k: 'angstrom^-1', r: 'angstrom', chi: 'dimensionless', chirMag: `angstrom^(-${data.settings.kweight + 1})` },
          notes: ['Other inferred normalization and background settings are not exposed by this binding.', 'Fourier peaks are not phase-corrected bond distances.'],
          processingMilliseconds: performance.now() - started,
        },
      };
    } finally { spectrum.free(); }
    port.postMessage({ id: data.id, type: 'result', result },
      [result.energy, result.mu, result.norm, result.flat, result.k, result.chi, result.r, result.chirMag].map(array => array.buffer as ArrayBuffer));
  } catch (error) {
    port.postMessage({ id: data.id, type: 'error', message: error instanceof Error ? error.message : String(error) });
  }
};
