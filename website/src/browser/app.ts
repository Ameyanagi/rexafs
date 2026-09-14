import type { SpectrumMapping } from '../../../js-rexafs/types';
import { MAX_SOURCE_BYTES, validateSettings } from './input';
import type { AnalysisRequest, AnalysisResponse, AnalysisResult, AnalysisSettings, InspectionRequest, InspectionResponse, MeasurementPreview } from './protocol';
import { renderPlot } from './plot';

function get<T extends Element>(selector: string): T {
  const node = document.querySelector<T>(selector);
  if (!node) throw new Error(`Missing browser workspace control: ${selector}`);
  return node;
}

const app = get<HTMLElement>('#browser-app');
const baseUrl = app.dataset.base || '/';
const fileInput = get<HTMLInputElement>('#source-file');
const exampleButton = get<HTMLButtonElement>('#load-example');
const form = get<HTMLFormElement>('#analysis-form');
const mapping = get<HTMLFieldSetElement>('#data-mapping');
const energyColumn = get<HTMLSelectElement>('#energy-column');
const signalColumn = get<HTMLSelectElement>('#signal-column');
const referenceColumn = get<HTMLSelectElement>('#reference-column');
const quantity = get<HTMLSelectElement>('#signal-quantity');
const energyUnit = get<HTMLSelectElement>('#energy-unit');
const scanSelect = get<HTMLSelectElement>('#source-scan');
const signalSelect = get<HTMLSelectElement>('#source-signal');
const detectorColumns = get<HTMLSelectElement>('#detector-columns');
const datasetPaths = get<HTMLSelectElement>('#dataset-paths');
const customMapping = get<HTMLDetailsElement>('#custom-mapping');
let mappingConfirmed = false;
const processButton = get<HTMLButtonElement>('#process');
const cancelButton = get<HTMLButtonElement>('#cancel');
const csvButton = get<HTMLButtonElement>('#export-csv');
const provenanceButton = get<HTMLButtonElement>('#export-provenance');
const status = get<HTMLElement>('#analysis-status');
const error = get<HTMLElement>('#analysis-error');
const stale = get<HTMLElement>('#result-stale');
const workspace = get<HTMLElement>('#workspace');
const plot = get<SVGSVGElement>('#plot');
const plotPanel = get<HTMLElement>('#plot-panel');
const plotEmpty = get<HTMLElement>('#plot-empty');
const plotSummary = get<HTMLElement>('#plot-summary');
const tabs = [...document.querySelectorAll<HTMLButtonElement>('[data-view]')];
type View = 'raw' | 'normalized' | 'chi' | 'fourier';

let view: View = 'raw';
let source: AnalysisRequest['source'] | undefined;
let columnCount = 0;
let revision = 0;
let resultRevision = -1;
let result: AnalysisResult | undefined;
let completedRequest: AnalysisRequest | undefined;
let activeRequest: AnalysisRequest | undefined;
let worker: Worker | undefined;
let requestId = 0;
let loading = false;
let processing = false;
let sourceOperation = 0;
let fetchController: AbortController | undefined;
let inspectionWorker: Worker | undefined;
let preview: MeasurementPreview | undefined;
let selectedPaths: string[] | undefined;

function clearError(): void { error.hidden = true; error.textContent = ''; }
function showError(message: string): void { error.textContent = message; error.hidden = false; }
function message(cause: unknown): string { return cause instanceof Error ? cause.message : String(cause); }

function syncControls(): void {
  processButton.disabled = !source || columnCount < 2 || !mappingConfirmed || loading || processing;
  cancelButton.disabled = !loading && !processing;
  exampleButton.disabled = loading;
  workspace.setAttribute('aria-busy', String(loading || processing));
  const current = Boolean(result && resultRevision === revision);
  csvButton.disabled = !current || processing || loading;
  provenanceButton.disabled = !current || processing || loading;
  stale.hidden = !result || current;
}

function stopWorker(): void {
  worker?.terminate();
  worker = undefined;
  activeRequest = undefined;
  processing = false;
}

function markChanged(): void {
  revision++;
  stopWorker();
  clearError();
  status.textContent = source ? 'Settings changed. Process the spectrum to apply them.' : 'Load a spectrum to begin.';
  syncControls();
}

function updateQuantity(suggestColumns = false): void {
  const transmission = quantity.value !== 'mu';
  get<HTMLElement>('#detector-fields').hidden = quantity.value !== 'ratio';
  get<HTMLElement>('#bragg-fields').hidden = energyUnit.value !== 'bragg';
  get<HTMLElement>('#energy-origin-field').hidden = energyUnit.value !== 'offset_ev';
  get<HTMLElement>('#reference-field').hidden = !transmission;
  get<HTMLElement>('#transmission-help').hidden = !transmission;
  get<HTMLElement>('#signal-column-label').textContent = transmission ? 'Transmitted Iₜ column' : 'Absorption column';
  if (suggestColumns) {
    const available = Array.from({ length: columnCount }, (_, index) => index).filter(index => index !== Number(energyColumn.value));
    referenceColumn.value = String(available[0] ?? 1);
    signalColumn.value = String(available[transmission ? 1 : 0] ?? 1);
  }
}

function configureColumns(count: number): void {
  columnCount = count;
  for (const select of [energyColumn, signalColumn, referenceColumn, detectorColumns]) {
    select.replaceChildren(...Array.from({ length: count }, (_, index) => new Option(`${index + 1}: ${preview?.document.scans[Number(scanSelect.value)]?.columns[index]?.name ?? 'Column'} [${preview?.document.scans[Number(scanSelect.value)]?.columns[index]?.units ?? 'undeclared'}]`, String(index))));
  }
  energyColumn.value = '0';
  signalColumn.value = '1';
  referenceColumn.value = '1';
  quantity.value = 'mu';
  energyUnit.value = 'eV';
  const transmission = quantity.querySelector<HTMLOptionElement>('option[value="transmission"]');
  if (transmission) transmission.disabled = count < 3;
  updateQuantity();
  mapping.disabled = false;
}

function beginSourceLoad(): number {
  sourceOperation++;
  inspectionWorker?.terminate();
  inspectionWorker = undefined;
  fetchController?.abort();
  stopWorker();
  loading = true;
  source = undefined;
  preview = undefined;
  selectedPaths = undefined;
  mappingConfirmed = false;
  get<HTMLElement>('#mapping-summary').textContent = '';
  columnCount = 0;
  result = undefined;
  completedRequest = undefined;
  revision++;
  mapping.disabled = true;
  clearError();
  get<HTMLElement>('#source-name').textContent = 'Loading spectrum…';
  get<HTMLElement>('#source-details').textContent = '';
  get<HTMLElement>('#result-e0').textContent = '';
  get<HTMLElement>('#result-count').textContent = '';
  status.textContent = 'Reading spectrum…';
  drawView();
  syncControls();
  return sourceOperation;
}

async function acceptSource(name: string, bytes: Uint8Array, operation: number): Promise<void> {
  if (operation !== sourceOperation) return;
  const input = {name,bytes};
  const info = await inspectMeasurement(input, operation);
  if (operation !== sourceOperation) return;
  source = input;
  preview = info;
  configureScans();
  loading = false;
  get<HTMLElement>('#source-name').textContent = name;
  get<HTMLElement>('#source-details').textContent = `${info.document.format} · ${info.document.scans.length} scans · ${info.document.datasets.length} datasets`;
  status.textContent = columnCount ? 'Ready. Review the scan, signal and units, then process.' : 'No absorption channels selected. Inspect the datasets and diagnostics.';
  syncControls();
}

function inspectMeasurement(input: AnalysisRequest['source'], operation: number, paths?: string[]): Promise<MeasurementPreview> {
  inspectionWorker?.terminate();
  return new Promise((resolve, reject) => {
    const task = new Worker(new URL('./analysis.worker.ts', import.meta.url), {type:'module'});
    inspectionWorker = task;
    task.addEventListener('message', (event: MessageEvent<InspectionResponse>) => {
      if (event.data.id !== operation) return;
      if (event.data.type === 'inspection') { task.terminate(); if(inspectionWorker===task)inspectionWorker=undefined; resolve(event.data.preview); }
      else if (event.data.type === 'error') { task.terminate(); if(inspectionWorker===task)inspectionWorker=undefined; reject(new Error(event.data.message)); }
    });
    task.addEventListener('error', event => { task.terminate(); reject(new Error(event.message)); });
    const request: InspectionRequest = {id:operation,type:'inspect',source:input,baseUrl,datasetPaths:paths};
    task.postMessage(request);
  });
}

function configureScans(): void {
  if (!preview) return;
  scanSelect.replaceChildren(...preview.document.scans.map((scan,index)=>new Option(`${index+1}: ${scan.label} (${preview!.rowCounts[index]} points)`,String(index))));
  if (selectedPaths?.length) scanSelect.value=String(preview.document.scans.length-1);
  datasetPaths.replaceChildren(...preview.document.datasets.map(d=>{
    const option=new Option(`${d.path} [${d.shape.join(' × ')}]${d.imaginary != null ? ' · complex' : ''}`,d.path);
    option.disabled=d.shape.length!==1 || d.imaginary != null; option.selected=selectedPaths?.includes(d.path) ?? false;return option;
  }));
  get<HTMLElement>('#dataset-review').hidden=!preview.document.datasets.length;
  selectScan();
}

function selectScan(): void {
  const scan=preview?.document.scans[Number(scanSelect.value)];
  configureColumns(scan?.columns.length ?? 0);
  signalSelect.replaceChildren(new Option('Manual column selection','manual'),...(scan?.signals.map((s,i)=>new Option(s.name,String(i))) ?? []));
  mappingConfirmed = scan?.signals.length === 1;
  customMapping.open = !mappingConfirmed;
  if (scan?.signals.length===1) {signalSelect.value='0';applySignal(scan.signals[0].mapping);}
  get<HTMLElement>('#reader-warnings').textContent=[...(preview?.document.warnings ?? []),...(scan?.warnings ?? []),...(scan?.signals.length!==1 ? ['Review the original header and select the intended signal explicitly.'] : [])].join(' ');
  get<HTMLElement>('#reader-header').textContent=(scan?.header ?? '')+'\n'+(preview?.document.datasets.map(d=>`${d.path}: ${d.shape.join(' × ')} ${d.attributes.quantity ?? ''}`).join('\n') ?? '');
  mapping.disabled=false;
  updateMappingSummary();
  syncControls();
}

function updateMappingSummary(): void {
  const name = (select: HTMLSelectElement) => select.selectedOptions[0]?.textContent ?? 'Select a column';
  get<HTMLElement>('#mapping-summary').textContent = mappingConfirmed
    ? `Energy: ${name(energyColumn)} → eV. Signal: ${quantity.selectedOptions[0]?.textContent}.`
    : 'Choose a detected signal, or review and confirm the custom columns and units.';
}

function applySignal(m: SpectrumMapping): void {
  energyColumn.value=String(m.energy_column);
  energyUnit.value=m.energy.kind==='bragg'?'bragg':m.energy.kind==='offset_ev'?'offset_ev':m.energy.kind==='kev'?'keV':'eV';
  if (m.energy.kind==='offset_ev') get<HTMLInputElement>('#energy-origin').value=String(m.energy.offset_ev);
  if (m.energy.kind==='bragg') {
    get<HTMLInputElement>('#crystal-spacing').value=String(m.energy.d_spacing);
    get<HTMLInputElement>('#angle-scale').value=String(m.energy.degrees_per_unit);
  }
  quantity.value=m.signal.kind==='direct'?'mu':m.signal.kind;
  if (m.signal.kind==='direct') signalColumn.value=String(m.signal.column);
  else {
    referenceColumn.value=String(m.signal.incident);
    if (m.signal.kind==='transmission') signalColumn.value=String(m.signal.transmitted);
    else { const chosen=m.signal.detectors; for(const option of detectorColumns.options) option.selected=chosen.includes(Number(option.value)); }
  }
  updateQuantity();
}

function readMapping(): SpectrumMapping {
  return {
    energy_column:Number(energyColumn.value),
    energy:energyUnit.value==='bragg'?{kind:'bragg',d_spacing:Number(get<HTMLInputElement>('#crystal-spacing').value),degrees_per_unit:Number(get<HTMLInputElement>('#angle-scale').value)}:energyUnit.value==='offset_ev'?{kind:'offset_ev',offset_ev:Number(get<HTMLInputElement>('#energy-origin').value)}:{kind:energyUnit.value==='keV'?'kev':'ev'},
    signal:quantity.value==='mu'?{kind:'direct',column:Number(signalColumn.value)}:quantity.value==='transmission'?{kind:'transmission',incident:Number(referenceColumn.value),transmitted:Number(signalColumn.value)}:{kind:'ratio',incident:Number(referenceColumn.value),detectors:[...detectorColumns.selectedOptions].map(o=>Number(o.value))},
  };
}
scanSelect.addEventListener('change',()=>selectScan());
signalSelect.addEventListener('change',()=>{
  const candidate=preview?.document.scans[Number(scanSelect.value)]?.signals[Number(signalSelect.value)];
  if(candidate)applySignal(candidate.mapping);
  mappingConfirmed = true;
  customMapping.open = !candidate;
  updateMappingSummary(); syncControls();
});
energyUnit.addEventListener('change',()=>updateQuantity());
for(const control of [energyColumn,signalColumn,referenceColumn,quantity,energyUnit,detectorColumns,
  get<HTMLInputElement>('#crystal-spacing'),get<HTMLInputElement>('#angle-scale'),get<HTMLInputElement>('#energy-origin')]) control.addEventListener('change',()=>{
  signalSelect.value='manual'; mappingConfirmed=true; updateMappingSummary(); syncControls();
});
get<HTMLButtonElement>('#confirm-mapping').addEventListener('click',()=>{
  signalSelect.value='manual'; mappingConfirmed=true; updateMappingSummary(); markChanged();
});
get<HTMLButtonElement>('#use-datasets').addEventListener('click',async()=>{
  if (!source) return;
  const operation=sourceOperation;
  const paths=[...datasetPaths.selectedOptions].map(o=>o.value);
  if(paths.length<2){showError('Select at least two equal-length dataset vectors.');return;}
  loading=true;syncControls();
  try {const info=await inspectMeasurement(source,operation,paths);if(operation===sourceOperation){preview=info;selectedPaths=paths;configureScans();}}
  catch(cause){if(operation===sourceOperation)showError(message(cause));}
  finally{if(operation===sourceOperation){loading=false;syncControls();}}
});

function sourceFailed(cause: unknown, operation: number): void {
  if (operation !== sourceOperation) return;
  loading = false;
  get<HTMLElement>('#source-name').textContent = 'No spectrum loaded';
  status.textContent = 'The spectrum could not be loaded.';
  showError(message(cause));
  syncControls();
}

fileInput.addEventListener('change', async () => {
  const file = fileInput.files?.[0];
  if (!file) return;
  const operation = beginSourceLoad();
  try {
    if (file.size > MAX_SOURCE_BYTES) throw new Error('This browser preview accepts files up to 8 MiB.');
    await acceptSource(file.name, new Uint8Array(await file.arrayBuffer()), operation);
  } catch (cause) { sourceFailed(cause, operation); }
});

exampleButton.addEventListener('click', async () => {
  const operation = beginSourceLoad();
  form.reset();
  updateQuantity();
  fetchController = new AbortController();
  try {
    const response = await fetch(`${baseUrl}examples/cu_150k.xmu`, { signal: fetchController.signal });
    if (!response.ok) throw new Error(`The Cu example could not be downloaded (HTTP ${response.status}).`);
    const text = await response.text();
    if (operation !== sourceOperation) return;
    fileInput.value = '';
    await acceptSource('cu_150k.xmu', new TextEncoder().encode(text), operation);
  } catch (cause) { sourceFailed(cause, operation); }
});

form.addEventListener('input', () => markChanged());
quantity.addEventListener('change', () => updateQuantity(true));

function readSettings(): AnalysisSettings {
  const value = (id: string) => Number(get<HTMLInputElement | HTMLSelectElement>(`#setting-${id}`).value);
  const e0 = get<HTMLInputElement>('#setting-e0').value;
  return {
    ...(e0.trim() ? { e0: Number(e0) } : {}),
    rbkg: value('rbkg'), kmin: value('kmin'), kmax: value('kmax'),
    kweight: value('kweight'), dk: value('dk'), nfft: value('nfft'),
    grid: get<HTMLSelectElement>('#setting-grid').value as AnalysisSettings['grid'],
  };
}

form.addEventListener('submit', event => {
  event.preventDefault();
  if (!source || !mappingConfirmed || processing || loading || !form.reportValidity()) return;
  clearError();
  try {
    const settings = readSettings();
    validateSettings(settings);
    stopWorker();
    const request: AnalysisRequest = { id: ++requestId, type: 'process', source, selection: {scan:Number(scanSelect.value),mapping:readMapping(),datasetPaths:Number(scanSelect.value)===(preview?.document.scans.length ?? 0)-1?selectedPaths:undefined}, settings, baseUrl };
    const startedRevision = revision;
    activeRequest = request;
    processing = true;
    status.textContent = 'Starting the WebAssembly engine…';
    syncControls();
    worker = new Worker(new URL('./analysis.worker.ts', import.meta.url), { type: 'module' });
    worker.addEventListener('message', (event: MessageEvent<AnalysisResponse>) => {
      const response = event.data;
      if (activeRequest?.id !== response.id || startedRevision !== revision) return;
      if (response.type === 'progress') {
        status.textContent = response.stage;
      } else if (response.type === 'error') {
        stopWorker();
        status.textContent = 'Processing stopped. Check the input and settings.';
        showError(response.message);
        syncControls();
      } else {
        result = response.result;
        completedRequest = request;
        resultRevision = startedRevision;
        stopWorker();
        get<HTMLElement>('#result-e0').textContent = `E₀ ${result.e0.toFixed(2)} eV`;
        get<HTMLElement>('#result-count').textContent = `${result.energy.length.toLocaleString()} samples`;
        status.textContent = `Processed ${result.sourceName}.`;
        drawView();
        syncControls();
      }
    });
    worker.addEventListener('error', event => {
      if (activeRequest?.id !== request.id) return;
      event.preventDefault();
      stopWorker();
      status.textContent = 'The analysis engine could not finish.';
      showError(event.message || 'The WebAssembly worker could not start. Reload the page and try again.');
      syncControls();
    });
    worker.postMessage(request);
  } catch (cause) {
    stopWorker();
    showError(message(cause));
    status.textContent = 'Check the processing settings.';
    syncControls();
  }
});

cancelButton.addEventListener('click', () => {
  sourceOperation++;
  inspectionWorker?.terminate();
  inspectionWorker = undefined;
  fetchController?.abort();
  loading = false;
  stopWorker();
  clearError();
  if (!source) get<HTMLElement>('#source-name').textContent = 'No spectrum loaded';
  status.textContent = 'Cancelled. You can load a spectrum or process again.';
  syncControls();
});

function currentCurve() {
  if (!result || !completedRequest) return undefined;
  const fourierPower = completedRequest.settings.kweight + 1;
  const powers = ['⁰', '¹', '²', '³', '⁴'];
  const absorptionUnit = completedRequest.selection.mapping.signal.kind === 'transmission' ? 'dimensionless' : 'input units';
  switch (view) {
    case 'raw': return { x: result.energy, y: result.mu, title: 'Imported absorption', xLabel: 'Energy (eV)', yLabel: `μ(E) (${absorptionUnit})`, columns: ['energy_eV', completedRequest.selection.mapping.signal.kind === 'transmission' ? 'mu_dimensionless' : 'mu_input_units'], note: 'Energy is shown in eV. Absorption uses the selected signal columns.' };
    case 'normalized': return { x: result.energy, y: result.norm, title: 'Normalized absorption', xLabel: 'Energy (eV)', yLabel: 'Normalized μ(E)', columns: ['energy_eV', 'mu_normalized'], note: 'The pre-edge baseline is removed and the absorption edge step is scaled to 1.' };
    case 'chi': return { x: result.k, y: result.chi, title: 'Background-subtracted EXAFS', xLabel: 'k (Å⁻¹)', yLabel: 'χ(k)', columns: ['k_inverse_angstrom', 'chi'], note: 'χ(k) is dimensionless and shown without k weighting.' };
    case 'fourier': return { x: result.r, y: result.chirMag, title: 'Fourier-transform magnitude', xLabel: 'R (Å)', yLabel: `|χ(R)| (Å⁻${powers[fourierPower]})`, columns: ['R_angstrom', `chir_magnitude_inverse_angstrom_power_${fourierPower}`], zeroMinimum: true, note: `Transform of k${powers[completedRequest.settings.kweight]}χ(k). R peaks are not phase-corrected bond distances.` };
  }
}

function drawView(): void {
  const curve = currentCurve();
  plot.style.display = curve ? '' : 'none';
  plot.setAttribute('aria-hidden', String(!curve));
  plotEmpty.hidden = Boolean(curve);
  if (!curve || !result) {
    plot.replaceChildren();
    plotSummary.textContent = 'Normalization, background removal and Fourier transformation run in WebAssembly.';
    return;
  }
  try {
    renderPlot(plot, { ...curve, title: `${result.sourceName}: ${curve.title}` });
    plotSummary.textContent = `${curve.x.length.toLocaleString()} points. ${curve.note}`;
  } catch (cause) { showError(message(cause)); }
}

function selectView(tab: HTMLButtonElement, focus = false): void {
  view = tab.dataset.view as View;
  for (const button of tabs) {
    button.setAttribute('aria-selected', String(button === tab));
    button.tabIndex = button === tab ? 0 : -1;
  }
  plotPanel.setAttribute('aria-labelledby', tab.id);
  if (focus) tab.focus();
  drawView();
}

tabs.forEach((tab, index) => {
  tab.addEventListener('click', () => selectView(tab));
  tab.addEventListener('keydown', event => {
    let next: number;
    if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
    else if (event.key === 'ArrowLeft') next = (index + tabs.length - 1) % tabs.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = tabs.length - 1;
    else return;
    event.preventDefault();
    selectView(tabs[next], true);
  });
});

function save(content: string, suffix: string, mime: string): void {
  if (!result || resultRevision !== revision) return;
  const stem = result.sourceName.replace(/\.[^.]+$/, '').replace(/[^a-zA-Z0-9_-]+/g, '_').replace(/^_+|_+$/g, '').slice(0, 100) || 'spectrum';
  const blobUrl = URL.createObjectURL(new Blob([content], { type: mime }));
  const link = document.createElement('a');
  link.href = blobUrl;
  link.download = `${stem}_${suffix}`;
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(blobUrl), 1000);
}

csvButton.addEventListener('click', () => {
  const curve = currentCurve();
  if (!curve) return;
  const lines = [curve.columns.join(',')];
  for (let index = 0; index < curve.x.length; index++) lines.push(`${curve.x[index]},${curve.y[index]}`);
  save(`${lines.join('\n')}\n`, `${view}.csv`, 'text/csv;charset=utf-8');
});
provenanceButton.addEventListener('click', () => {
  if (result) save(`${JSON.stringify(result.provenance, null, 2)}\n`, 'provenance.json', 'application/json');
});

let plotWidth = 0;
new ResizeObserver(entries => {
  const width = entries[0].contentRect.width;
  if (Math.abs(width - plotWidth) > 1) { plotWidth = width; drawView(); }
}).observe(plotPanel);
syncControls();
