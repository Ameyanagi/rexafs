import { parseSource, validateSettings, validateNumericalWorkspace } from './input';
import type { AnalysisRequest, AnalysisResponse, AnalysisResult } from './protocol';

type Engine = typeof import('../../../js-rexafs/index');
interface EngineManifest { version: string; commit: string; dirty: boolean; wasmSha256: string; channel: string }
const port = self as unknown as {
  onmessage: ((event: MessageEvent<AnalysisRequest>) => void) | null;
  postMessage(message: AnalysisResponse, transfer?: Transferable[]): void;
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
async function sha256(text: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return hex(digest);
}

/** Each request owns a fresh spectrum; all native allocations are released on success or failure. */
port.onmessage = async ({ data }) => {
  if (data.type !== 'process') return;
  const progress = (stage: string) => port.postMessage({ id: data.id, type: 'progress', stage });
  try {
    validateSettings(data.settings);
    const { energy, mu } = parseSource(data.source.text, data.columns);
    progress('Loading engine');
    const { engine, manifest } = await loadEngine(data.baseUrl);
    const inputHash = await sha256(data.source.text);
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
          schema: 'rexafs-browser-analysis-v1', createdAt: new Date().toISOString(),
          engine: manifest, input: { name: data.source.name, textSha256: inputHash, encoding: 'UTF-8 text as imported', rows: energy.length, columns: data.columns },
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
