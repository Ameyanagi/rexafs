import release from '../data/refeff-release.json';
import { renderPlot } from './plot';
import { parseScatteringChi, SCATTERING_LIMITS as limits, validateWorkspacePath } from './scattering-input';

interface EngineManifest {
  version: string;
  tag: string;
  sourceCommit: string;
  cliVersion: string;
  target: string;
  archiveSha256: string;
  wasmSha256: string;
  files: Record<string, { sha256: string; bytes: number }>;
  [key: string]: unknown;
}
interface FeffResult {
  exitCode: number;
  report: { data?: { atoms?: number }; error?: { message?: string }; [key: string]: unknown };
  files: Record<string, Uint8Array>;
  stdout: string;
  stderr: string;
}
interface Adapter {
  runFeff(options: {
    input: string;
    files: Record<string, Uint8Array>;
    signal: AbortSignal;
    wasmUrl: string;
    onLog(event: { stream: string; text: string }): void;
  }): Promise<FeffResult>;
}
interface FileRecord { name: string; bytes: number; sha256: string; encoding: string; content?: string; text?: string; sourceName?: string }
interface CalculationRecord {
  schemaVersion: number;
  engine: EngineManifest;
  startedAt: string;
  completedAt: string;
  timing: { elapsedSeconds: number; boundary: string };
  inputs: { root: FileRecord; auxiliaries: FileRecord[] };
  calculation: { exitCode: number; report: FeffResult['report'] };
  outputs: Record<string, { bytes: number; sha256: string }>;
  limits: typeof limits;
}

function get<T extends Element>(selector: string): T {
  const node = document.querySelector<T>(selector);
  if (!node) throw new Error(`Missing scattering workspace control: ${selector}`);
  return node;
}
const app = get<HTMLElement>('#scattering-app');
const engineBase = `${app.dataset.base || '/'}refeff/`;
const editor = get<HTMLTextAreaElement>('#feff-input');
const inputFile = get<HTMLInputElement>('#feff-file');
const exampleButton = get<HTMLButtonElement>('#load-znse');
const auxiliaryInput = get<HTMLInputElement>('#aux-files');
const clearAuxButton = get<HTMLButtonElement>('#clear-aux');
const runButton = get<HTMLButtonElement>('#run-feff');
const cancelButton = get<HTMLButtonElement>('#cancel-feff');
const status = get<HTMLElement>('#scattering-status');
const errorBox = get<HTMLElement>('#scattering-error');
const stale = get<HTMLElement>('#scattering-stale');
const workspace = get<HTMLElement>('#scattering-workspace');
const svg = get<SVGSVGElement>('#scattering-plot');
const empty = get<HTMLElement>('#scattering-empty');
const outputSelect = get<HTMLSelectElement>('#output-file');
const downloadButton = get<HTMLButtonElement>('#download-output');
const recordButton = get<HTMLButtonElement>('#download-scattering-record');
const log = get<HTMLElement>('#scattering-log');
const encoder = new TextEncoder();
let revision = 0;
let completedRevision = -1;
let operation = 0;
let busy = false;
let controller: AbortController | undefined;
let sourceName = 'feff.inp';
let auxiliaryFiles: Record<string, Uint8Array> = Object.create(null);
let result: FeffResult | undefined;
let record: CalculationRecord | undefined;
let plotData: ReturnType<typeof parseScatteringChi> | undefined;
const downloadUrls = new Set<string>();

function message(cause: unknown): string { return cause instanceof Error ? cause.message : String(cause); }
function showError(text: string): void { errorBox.textContent = text; errorBox.hidden = false; }
function clearError(): void { errorBox.textContent = ''; errorBox.hidden = true; }
function assertActive(id: number, signal: AbortSignal): void {
  if (id !== operation || signal.aborted) throw signal.reason ?? new DOMException('Calculation cancelled.', 'AbortError');
}
function sync(): void {
  const current = Boolean(record && completedRevision === revision);
  runButton.disabled = busy || !editor.value.trim();
  cancelButton.disabled = !busy;
  downloadButton.disabled = !current || busy || !outputSelect.value;
  outputSelect.disabled = !current || busy || !Object.keys(result?.files ?? {}).length;
  recordButton.disabled = !current || busy;
  clearAuxButton.disabled = !Object.keys(auxiliaryFiles).length;
  workspace.setAttribute('aria-busy', String(busy));
  stale.hidden = !record || current;
}
function stop(): void {
  if (busy && !record) {
    empty.querySelector('h3')!.textContent = 'Ready to calculate.';
    empty.querySelector('p')!.textContent = 'Review the input and run again when ready.';
  }
  operation++;
  controller?.abort();
  controller = undefined;
  busy = false;
}
function invalidate(): void {
  stop();
  revision++;
  clearError();
  status.textContent = editor.value.trim() ? 'Input changed. Run the calculation to apply it.' : 'Enter a FEFF input or load the ZnSe test input.';
  sync();
}
function beginLoading(text: string): { id: number; signal: AbortSignal } {
  invalidate();
  controller = new AbortController();
  busy = true;
  status.textContent = text;
  sync();
  return { id: operation, signal: controller.signal };
}
function finishLoading(id: number, cause?: unknown): void {
  if (id !== operation) return;
  busy = false;
  controller = undefined;
  if (cause) { status.textContent = 'The input could not be loaded.'; showError(message(cause)); }
  else status.textContent = 'Ready. Review the input, then run the calculation.';
  sync();
}
function setSource(name: string, text: string): void {
  if (encoder.encode(text).byteLength > limits.inputBytes) throw new Error('The FEFF input exceeds 1 MiB.');
  editor.value = text;
  sourceName = name;
  get<HTMLElement>('#feff-source-name').textContent = name;
}
function showAuxiliaries(): void {
  get<HTMLElement>('#aux-list').replaceChildren(...Object.entries(auxiliaryFiles).map(([name, bytes]) => {
    const item = document.createElement('li');
    item.textContent = `${name} · ${bytes.byteLength.toLocaleString()} bytes`;
    return item;
  }));
  sync();
}
async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes.slice().buffer);
  return Array.from(new Uint8Array(digest), value => value.toString(16).padStart(2, '0')).join('');
}
async function fetchBytes(url: string, maximum: number, signal: AbortSignal): Promise<Uint8Array> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Could not load ${url.split('/').at(-1)} (HTTP ${response.status}).`);
  if (Number(response.headers.get('content-length')) > maximum) throw new Error('The downloaded file exceeds its size limit.');
  if (!response.body) throw new Error('The browser did not provide a readable download.');
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let count = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      count += value.byteLength;
      if (count > maximum) throw new Error('The downloaded file exceeds its size limit.');
      chunks.push(value);
    }
  } finally { await reader.cancel(); reader.releaseLock(); }
  const bytes = new Uint8Array(count);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
  return bytes;
}
async function loadManifest(signal: AbortSignal): Promise<EngineManifest> {
  const bytes = await fetchBytes(`${engineBase}manifest.json`, 128 * 1024, signal);
  const manifest = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)) as EngineManifest;
  for (const key of ['version', 'tag', 'sourceCommit', 'cliVersion', 'target', 'archiveSha256', 'wasmSha256'] as const) {
    if (manifest?.[key] !== release[key]) throw new Error('The ReFEFF assets do not match this browser workspace. Reload after the deployment completes.');
  }
  for (const name of ['refeff.wasm', 'index.mjs', 'worker.mjs', 'znse.inp']) {
    const file = manifest.files?.[name];
    if (!file || !/^[a-f0-9]{64}$/.test(file.sha256) || !Number.isSafeInteger(file.bytes) || file.bytes <= 0) throw new Error(`The ReFEFF manifest is missing a valid ${name} fingerprint.`);
  }
  if (manifest.files['refeff.wasm'].sha256 !== release.wasmSha256 || manifest.files['refeff.wasm'].bytes !== release.wasmBytes) throw new Error('The WebAssembly fingerprint does not match the pinned ReFEFF release.');
  return manifest;
}

editor.addEventListener('input', () => { invalidate(); get<HTMLElement>('#feff-source-name').textContent = `${sourceName} · edited`; });
inputFile.addEventListener('change', async () => {
  const file = inputFile.files?.[0];
  if (!file) return;
  const { id, signal } = beginLoading('Reading FEFF input…');
  try {
    if (file.size > limits.inputBytes) throw new Error('The FEFF input exceeds 1 MiB.');
    const bytes = new Uint8Array(await file.arrayBuffer());
    assertActive(id, signal);
    setSource(file.name, new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    finishLoading(id);
  } catch (cause) { finishLoading(id, cause); }
  finally { inputFile.value = ''; }
});
exampleButton.addEventListener('click', async () => {
  const { id, signal } = beginLoading('Loading the ZnSe test input…');
  try {
    const manifest = await loadManifest(signal);
    const bytes = await fetchBytes(`${engineBase}znse.inp`, limits.inputBytes, signal);
    if (bytes.byteLength !== manifest.files['znse.inp'].bytes || await sha256(bytes) !== manifest.files['znse.inp'].sha256) throw new Error('The ZnSe example does not match its release fingerprint.');
    assertActive(id, signal);
    setSource(`ReFEFF ${release.version} ZnSe test input · includes Kr`, new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    auxiliaryFiles = Object.create(null);
    showAuxiliaries();
    finishLoading(id);
  } catch (cause) { finishLoading(id, cause); }
});
auxiliaryInput.addEventListener('change', async () => {
  const files = Array.from(auxiliaryInput.files ?? []);
  if (!files.length) return;
  const { id, signal } = beginLoading('Reading auxiliary files…');
  try {
    const next: Record<string, Uint8Array> = Object.assign(Object.create(null), auxiliaryFiles);
    const incoming = new Set<string>();
    for (const file of files) {
      validateWorkspacePath(file.name);
      if (file.name.includes('/') || file.name === 'feff.inp') throw new Error('Auxiliary files need a root filename other than feff.inp.');
      if (incoming.has(file.name)) throw new Error(`More than one selected file is named ${file.name}.`);
      incoming.add(file.name);
      if (file.size > limits.totalInputBytes) throw new Error('An auxiliary file exceeds the 8 MiB input limit.');
      const bytes = new Uint8Array(await file.arrayBuffer());
      assertActive(id, signal);
      next[file.name] = bytes;
      if (Object.keys(next).length > limits.auxiliaryFiles) throw new Error('Use at most 64 auxiliary files.');
      if (encoder.encode(editor.value).byteLength + Object.values(next).reduce((sum, item) => sum + item.byteLength, 0) > limits.totalInputBytes) throw new Error('The FEFF input and auxiliary files exceed 8 MiB in total.');
    }
    auxiliaryFiles = next;
    showAuxiliaries();
    finishLoading(id);
  } catch (cause) { finishLoading(id, cause); }
  finally { auxiliaryInput.value = ''; }
});
clearAuxButton.addEventListener('click', () => { invalidate(); auxiliaryFiles = Object.create(null); showAuxiliaries(); });
cancelButton.addEventListener('click', () => { stop(); status.textContent = 'Cancelled. Edit the input or run again.'; sync(); });

function draw(): void {
  svg.style.display = plotData ? 'block' : 'none';
  svg.setAttribute('aria-hidden', String(!plotData));
  empty.hidden = Boolean(plotData);
  if (plotData) renderPlot(svg, { ...plotData, title: 'Calculated EXAFS χ(k)', xLabel: 'k (Å⁻¹)', yLabel: 'χ(k) (dimensionless)' });
}
function base64(bytes: Uint8Array): string {
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 8192) binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  return btoa(binary);
}
function validateResult(value: FeffResult): void {
  if (!value || !Number.isInteger(value.exitCode) || !value.report || typeof value.report !== 'object' || !value.files || typeof value.files !== 'object') throw new Error('ReFEFF returned an invalid calculation result.');
  const files = Object.entries(value.files);
  if (files.length > limits.outputFiles) throw new Error('The calculation returned more than 10,000 files. Use the desktop or native API for this input.');
  let bytes = 0;
  for (const [name, contents] of files) {
    validateWorkspacePath(name);
    if (!(contents instanceof Uint8Array)) throw new Error(`ReFEFF returned invalid bytes for ${name}.`);
    bytes += contents.byteLength;
    if (bytes > limits.outputBytes) throw new Error('Returned files exceed 128 MiB. Use the desktop or native API for this input.');
  }
}
async function calculate(): Promise<void> {
  if (busy) return;
  clearError();
  const input = editor.value;
  const inputBytes = encoder.encode(input);
  const files = { ...auxiliaryFiles };
  if (!input.trim()) { showError('Enter a FEFF input first.'); return; }
  if (inputBytes.byteLength > limits.inputBytes) { showError('The FEFF input exceeds 1 MiB.'); return; }
  if (inputBytes.byteLength + Object.values(files).reduce((sum, value) => sum + value.byteLength, 0) > limits.totalInputBytes) { showError('The FEFF input and auxiliary files exceed 8 MiB in total.'); return; }
  if (!globalThis.WebAssembly || !globalThis.Worker || !globalThis.crypto?.subtle) { showError('Use a browser with WebAssembly, Web Workers and secure-context Web Crypto support.'); return; }
  const id = ++operation;
  const sourceRevision = revision;
  const inputSourceName = sourceName;
  controller = new AbortController();
  const signal = controller.signal;
  const runController = controller;
  busy = true;
  result = undefined; record = undefined; plotData = undefined;
  outputSelect.replaceChildren(new Option('Calculation in progress', ''));
  get<HTMLElement>('#scattering-summary').textContent = '';
  get<HTMLElement>('#scattering-log-summary').textContent = '';
  get<HTMLElement>('#scattering-plot-note').textContent = 'The plot uses chi.dat columns 1–2: k (Å⁻¹) and dimensionless χ(k).';
  empty.querySelector('h3')!.textContent = 'Calculating scattering…';
  empty.querySelector('p')!.textContent = 'Progress and generated files appear here.';
  log.textContent = '';
  status.textContent = 'Loading and verifying ReFEFF WebAssembly…';
  draw(); sync();
  const start = performance.now();
  const startedAt = new Date().toISOString();
  let wasmUrl: string | undefined;
  let receivedLogBytes = 0;
  let logTruncated = false;
  const timeout = setTimeout(() => runController.abort(new Error('The calculation exceeded the 5-minute browser time limit.')), limits.runtimeMilliseconds);
  try {
    const manifest = await loadManifest(signal);
    const bytes = await fetchBytes(`${engineBase}refeff.wasm`, release.wasmBytes, signal);
    if (bytes.byteLength !== release.wasmBytes || await sha256(bytes) !== release.wasmSha256) throw new Error('The downloaded WebAssembly does not match the ReFEFF release SHA-256 fingerprint.');
    assertActive(id, signal);
    const moduleUrl = `${engineBase}index.mjs`;
    const adapter = await import(/* @vite-ignore */ moduleUrl) as Adapter;
    assertActive(id, signal);
    if (typeof adapter.runFeff !== 'function') throw new Error('The ReFEFF browser adapter could not be loaded.');
    wasmUrl = URL.createObjectURL(new Blob([bytes.slice().buffer], { type: 'application/wasm' }));
    status.textContent = 'Calculating with ReFEFF on one worker thread…';
    const calculated = await adapter.runFeff({
      input, files, signal, wasmUrl,
      onLog: ({ stream, text }) => {
        assertActive(id, signal);
        receivedLogBytes += encoder.encode(text).byteLength;
        if (receivedLogBytes > limits.receivedLogBytes) {
          const cause = new Error('The calculation exceeded the 2 MiB progress-text limit.');
          runController.abort(cause);
          throw cause;
        }
        if (stream !== 'stderr') return;
        const tail = `${log.textContent ?? ''}${text}`;
        logTruncated ||= tail.length > limits.logCharacters;
        log.textContent = tail.slice(-limits.logCharacters);
        get<HTMLElement>('#scattering-log-summary').textContent = logTruncated ? 'latest 32,768 characters' : 'live';
        log.scrollTop = log.scrollHeight;
      },
    });
    assertActive(id, signal);
    validateResult(calculated);
    status.textContent = 'Preparing output fingerprints…';
    const root: FileRecord = { name: 'feff.inp', sourceName: inputSourceName, encoding: 'utf-8', text: input, bytes: inputBytes.byteLength, sha256: await sha256(inputBytes) };
    const auxiliaries: FileRecord[] = [];
    for (const [name, contents] of Object.entries(files)) {
      assertActive(id, signal);
      auxiliaries.push({ name, bytes: contents.byteLength, sha256: await sha256(contents), encoding: 'base64', content: base64(contents) });
    }
    const outputs: CalculationRecord['outputs'] = Object.create(null);
    for (const [name, contents] of Object.entries(calculated.files)) {
      assertActive(id, signal);
      outputs[name] = { bytes: contents.byteLength, sha256: await sha256(contents) };
    }
    assertActive(id, signal);
    const elapsedSeconds = (performance.now() - start) / 1000;
    record = { schemaVersion: 1, engine: manifest, startedAt, completedAt: new Date().toISOString(), timing: { elapsedSeconds, boundary: 'Includes engine download, verification, compilation, calculation and output fingerprints.' }, inputs: { root, auxiliaries }, calculation: { exitCode: calculated.exitCode, report: calculated.report }, outputs, limits };
    result = calculated;
    completedRevision = sourceRevision;
    const names = Object.keys(calculated.files).sort((left, right) => {
      const order = (name: string) => name === 'chi.dat' ? 0 : name === 'xmu.dat' ? 1 : 2;
      return order(left) - order(right) || left.localeCompare(right);
    });
    outputSelect.replaceChildren(...(names.length ? names.map(name => new Option(`${name} · ${calculated.files[name].byteLength.toLocaleString()} bytes`, name)) : [new Option('No generated files', '')]));
    get<HTMLElement>('#scattering-summary').textContent = `${calculated.report.data?.atoms === undefined ? '' : `${calculated.report.data.atoms} atoms · `}${names.length} files · ${elapsedSeconds.toFixed(1)} s`;
    get<HTMLElement>('#scattering-log-summary').textContent = logTruncated ? 'latest 32,768 characters' : receivedLogBytes ? 'complete' : 'no progress text';
    if (calculated.exitCode !== 0) {
      status.textContent = 'Calculation failed. The report and any partial files are available below.';
      showError(calculated.report.error?.message || `ReFEFF exited with code ${calculated.exitCode}.`);
      empty.querySelector('h3')!.textContent = 'The calculation did not finish.';
      empty.querySelector('p')!.textContent = 'Check the error and calculation log before trying again.';
    } else {
      status.textContent = 'Calculation complete. Download the results or save the calculation record.';
      empty.querySelector('h3')!.textContent = 'Calculation complete.';
      empty.querySelector('p')!.textContent = 'This input produced no chi.dat spectrum. Its generated files are available below.';
      if (calculated.files['chi.dat']) {
        try {
          plotData = parseScatteringChi(calculated.files['chi.dat']);
          get<HTMLElement>('#scattering-plot-note').textContent = `${plotData.x.length.toLocaleString()} points from chi.dat. k is in Å⁻¹; χ(k) is dimensionless.`;
        } catch (cause) {
          showError(`The calculation finished, but χ(k) could not be plotted: ${message(cause)}`);
          empty.querySelector('p')!.textContent = 'Download chi.dat to inspect the original output.';
        }
      }
    }
    draw();
  } catch (cause) {
    if (id !== operation) return;
    status.textContent = 'The calculation could not finish.';
    showError(message(cause));
    empty.querySelector('h3')!.textContent = 'The calculation did not finish.';
    empty.querySelector('p')!.textContent = 'Check the error, then try again.';
    outputSelect.replaceChildren(new Option('No completed calculation', ''));
  } finally {
    clearTimeout(timeout);
    if (wasmUrl) URL.revokeObjectURL(wasmUrl);
    if (id === operation) { busy = false; controller = undefined; sync(); }
  }
}
runButton.addEventListener('click', () => { void calculate(); });
outputSelect.addEventListener('change', sync);
function download(name: string, bytes: Uint8Array | string, type: string): void {
  const blob = new Blob([typeof bytes === 'string' ? bytes : bytes.slice().buffer], { type });
  const url = URL.createObjectURL(blob);
  downloadUrls.add(url);
  const link = document.createElement('a');
  link.href = url; link.download = name; link.hidden = true;
  document.body.append(link); link.click(); link.remove();
  setTimeout(() => { URL.revokeObjectURL(url); downloadUrls.delete(url); }, 1000);
}
downloadButton.addEventListener('click', () => {
  if (busy || !result || completedRevision !== revision) return;
  const name = outputSelect.value;
  const bytes = result.files[name];
  if (bytes) download(name.replaceAll('/', '__'), bytes, 'application/octet-stream');
});
recordButton.addEventListener('click', () => {
  if (!busy && record && completedRevision === revision) download('rexafs-scattering-provenance.json', JSON.stringify(record, null, 2), 'application/json');
});
new ResizeObserver(() => { if (plotData) draw(); }).observe(svg.parentElement!);
window.addEventListener('pagehide', () => { stop(); for (const url of downloadUrls) URL.revokeObjectURL(url); downloadUrls.clear(); });
sync();
