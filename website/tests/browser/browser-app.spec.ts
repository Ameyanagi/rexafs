import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { readFileSync } from 'node:fs';
import { Spectrum } from '../../../js-rexafs/node.js';
const base = (process.env.SITE_BASE || '/rexafs').replace(/\/$/, '');
const text = readFileSync(new URL('../../public/examples/cu_150k.xmu', import.meta.url), 'utf8');
const rows = text.split(/\r?\n/).map(line => line.split('#')[0].trim()).filter(Boolean).map(line => line.split(/\s+/).map(Number));
const energy = Float64Array.from(rows, row => row[0]);
const mu = Float64Array.from(rows, row => row[1]);
const spectrum = new Spectrum(energy, mu);
const reference = (() => {
  try {
    spectrum.fft();
    return { e0: spectrum.e0()!, r: spectrum.r()!, chir: spectrum.chir_mag()! };
  } finally { spectrum.free(); }
})();

test('browser workspace processes the real Cu fixture, exports Node-equivalent data and invalidates stale exports', async ({ page }) => {
  const errors: string[] = [];
  const uploads: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (request.method() !== 'GET') uploads.push(request.url()); });
  await page.goto(base + '/app/');
  await page.locator('#load-example').click();
  await expect(page.locator('#process')).toBeEnabled();
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed cu_150k.xmu.');
  await expect(page.locator('#result-e0')).toHaveText(`E₀ ${reference.e0.toFixed(2)} eV`);
  await expect(page.locator('#plot .data-line')).toHaveAttribute('points', /\d/);
  await page.locator('#view-fourier').click();
  const csvDownload = page.waitForEvent('download');
  await page.locator('#export-csv').click();
  const csv = readFileSync((await (await csvDownload).path())!, 'utf8').trim().split('\n');
  expect(csv[0]).toContain('power_3');
  expect(csv.length - 1).toBe(reference.r.length);
  for (const index of [0, 20, 50, 100]) {
    const [r, amplitude] = csv[index + 1].split(',').map(Number);
    expect(Math.abs(r - reference.r[index])).toBeLessThan(1e-12);
    expect(Math.abs(amplitude - reference.chir[index])).toBeLessThan(1e-9);
  }
  const jsonDownload = page.waitForEvent('download');
  await page.locator('#export-provenance').click();
  const record = JSON.parse(readFileSync((await (await jsonDownload).path())!, 'utf8'));
  expect(record.resolved.e0).toBe(reference.e0);
  expect(record.engine.wasmSha256).toMatch(/^[a-f0-9]{64}$/);
  expect(typeof record.engine.dirty).toBe('boolean');
  expect(record.input.bytesSha256).toMatch(/^[a-f0-9]{64}$/);
  expect(record.requested).toMatchObject({ rbkg: 1, kweight: 2, nfft: 2048 });
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: 'test-results/browser-workspace-desktop.png', fullPage: true });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.locator('#setting-kweight').fill('3');
  await expect(page.locator('#result-stale')).toBeVisible();
  await expect(page.locator('#export-csv')).toBeDisabled();
  await expect(page.locator('#export-provenance')).toBeDisabled();
  await page.locator('#process').click();
  await expect(page.locator('#export-csv')).toBeEnabled();
  await expect(page.locator('#result-stale')).toBeHidden();
  await expect(page.locator('#plot')).toContainText('Å⁻⁴');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: 'test-results/browser-workspace-mobile.png', fullPage: true });
  expect(errors).toEqual([]);
  expect(uploads).toEqual([]);
});

test('browser import respects transmission columns and keV, then reports invalid input and recovers', async ({ page }) => {
  await page.goto(base + '/app/');
  const transmission = '# energy_keV,I0,It\n' + rows.map(row => `${row[0] / 1000},10,${10 * Math.exp(-row[1])}`).join('\n');
  await page.locator('#source-file').setInputFiles({ name: 'transmission.csv', mimeType: 'text/csv', buffer: Buffer.from(transmission) });
  await expect(page.locator('#source-details')).toContainText('text · 1 scans');
  await expect(page.locator('#process')).toBeEnabled();
  await expect(page.locator('#custom-mapping')).not.toHaveAttribute('open', '');
  await page.locator('#custom-mapping summary').click();
  await page.locator('#signal-quantity').selectOption('transmission');
  await page.locator('#energy-unit').selectOption('keV');
  await expect(page.locator('#reference-column')).toHaveValue('1');
  await expect(page.locator('#signal-column')).toHaveValue('2');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed transmission.csv.');
  await expect(page.locator('#result-e0')).toHaveText(`E₀ ${reference.e0.toFixed(2)} eV`);
  await page.locator('#source-file').setInputFiles({ name: 'duplicate.dat', mimeType: 'text/plain', buffer: Buffer.from('2 0.5\n2 0.6\n') });
  await expect(page.locator('#export-csv')).toBeDisabled();
  await expect(page.locator('#process')).toBeDisabled();
  await page.locator('#confirm-mapping').click();
  await page.locator('#process').click();
  await expect(page.locator('#analysis-error')).toContainText('strictly increasing');
  await page.locator('#load-example').click();
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed cu_150k.xmu.');
  await expect(page.locator('#analysis-error')).toBeHidden();
});

test('cancelling loading destroys the Worker and the next calculation starts cleanly', async ({ page }) => {
  await page.route('**/wasm/manifest.json', async route => {
    await new Promise(resolve => setTimeout(resolve, 500));
    await route.continue().catch(() => {});
  });
  await page.goto(base + '/app/');
  await page.locator('#load-example').click();
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Loading engine');
  await page.locator('#cancel').click();
  await expect(page.locator('#analysis-status')).toContainText('Cancelled');
  await expect(page.locator('#process')).toBeEnabled();
  await expect(page.locator('#export-csv')).toBeDisabled();
  await page.unroute('**/wasm/manifest.json');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed cu_150k.xmu.');
});

test('short FFT lengths fail visibly instead of truncating data, then recover with enough points', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(base + '/app/');
  await page.locator('#load-example').click();
  for (const grid of ['Input', 'Larch']) {
    await page.locator('#setting-grid').selectOption(grid);
    await page.locator('#setting-nfft').selectOption('256');
    await page.locator('#process').click();
    await expect(page.locator('#analysis-error')).toContainText('without truncation');
    await expect(page.locator('#export-csv')).toBeDisabled();
    await expect(page.locator('#process')).toBeEnabled();
  }
  await page.locator('#setting-nfft').selectOption('2048');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed cu_150k.xmu.');
  await expect(page.locator('#analysis-error')).toBeHidden();
  await expect(page.locator('#export-csv')).toBeEnabled();
  expect(errors).toEqual([]);
});


test('shared reader exposes XTUNES project records and HDF5 detector shapes',async({page})=>{
  await page.goto(base+'/app/');
  const project=new URL('../../../crates/rexafs/tests/fixtures/sessions/xtunes/mixed-raw-analyzed.xtsp',import.meta.url);
  await page.locator('#source-file').setInputFiles({name:'mixed.xtsp',mimeType:'application/octet-stream',buffer:readFileSync(project)});
  await expect(page.locator('#source-details')).toContainText('xtunes_project · 2 scans');
  await expect(page.locator('#source-scan option')).toHaveCount(2);
  await page.locator('#source-scan').selectOption('1');
  await expect(page.locator('#source-signal')).toHaveValue('0');
  await expect(page.locator('#reader-warnings')).toContainText('Saved XTUNES');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed mixed.xtsp.');
  const h5=new URL('../../../crates/rexafs/tests/fixtures/xas/samples/max-iv/balder-unconfirmed/parseq-xas/scan-24212_eiger_streaming.h5',import.meta.url);
  await page.locator('#source-file').setInputFiles({name:'detector.h5',mimeType:'application/octet-stream',buffer:readFileSync(h5)});
  await expect(page.locator('#source-details')).toContainText('hdf5 · 0 scans');
  await expect(page.locator('#process')).toBeDisabled();
  await expect(page.locator('#dataset-review')).toBeVisible();
  expect(await page.locator('#dataset-paths option').count()).toBeGreaterThan(0);
});

test('generic HDF5 review validates shapes and builds a manually selected scan',async({page})=>{
  await page.goto(base+'/app/');
  const file=new URL('../../../crates/rexafs/tests/fixtures/xas/samples/max-iv/balder-unconfirmed/parseq-xas/20230318.h5',import.meta.url);
  await page.locator('#source-file').setInputFiles({name:'channels.h5',mimeType:'application/octet-stream',buffer:readFileSync(file)});
  await expect(page.locator('#source-details')).toContainText('hdf5');
  await page.locator('#dataset-review summary').click();
  await page.locator('#dataset-paths').selectOption(['/entry24212/measurement/mono1_energy_acs','/entry24213/measurement/albaem-01_ch1']);
  await page.locator('#use-datasets').click();
  await expect(page.locator('#analysis-error')).toContainText('matching');
  await page.locator('#dataset-paths').selectOption(['/entry24212/measurement/mono1_energy_acs','/entry24212/measurement/albaem-01_ch1']);
  await page.locator('#use-datasets').click();
  await expect(page.locator('#source-scan option:checked')).toContainText('Selected HDF5 datasets (601 points)');
  await expect(page.locator('#energy-column option')).toHaveCount(2);
  await page.locator('#energy-column').selectOption('1');
  await page.locator('#signal-column').selectOption('0');
  await expect(page.locator('#analysis-error')).toBeHidden();
  await expect(page.locator('#process')).toBeEnabled();
});

test('reference and multisample imports preserve selected energy and detector conversions', async ({ page }) => {
  await page.goto(base + '/app/');
  const referenceFile = new URL('../../../crates/rexafs/tests/fixtures/xas/samples/unspecified/fdmnes/xraylarch/FDMNES_2022_Mo2C_out.dat', import.meta.url);
  await page.locator('#source-file').setInputFiles({ name: 'reference.dat', mimeType: 'text/plain', buffer: readFileSync(referenceFile) });
  await expect(page.locator('#source-details')).toContainText('fdmnes');
  await expect(page.locator('#energy-unit')).toHaveValue('offset_ev');
  await expect(page.locator('#energy-origin')).toHaveValue('20000');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed reference.dat.');
  const download = page.waitForEvent('download');
  await page.locator('#export-provenance').click();
  const record = JSON.parse(readFileSync((await (await download).path())!, 'utf8'));
  expect(record.input.selection.mapping.energy).toEqual({ kind: 'offset_ev', offset_ev: 20000 });
  const multi = new URL('../../../crates/rexafs/tests/fixtures/xas/samples/nsls/x23a2/demeter/re4chan.000', import.meta.url);
  await page.locator('#source-file').setInputFiles({ name: 'four-samples.dat', mimeType: 'text/plain', buffer: readFileSync(multi) });
  await expect(page.locator('#source-signal option')).toHaveCount(5);
  await expect(page.locator('#process')).toBeDisabled();
  await page.locator('#source-signal').selectOption('3');
  await expect(page.locator('#process')).toBeEnabled();
  await expect(page.locator('#reference-column')).toHaveValue('4');
  await expect(page.locator('#signal-column')).toHaveValue('8');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed four-samples.dat.');
});

test('Larix imports select saved groups, expose complex results and retain duplicate-row errors', async ({ page }) => {
  await page.goto(base + '/app/');
  const root = new URL('../../../crates/rexafs/tests/fixtures/sessions/larix/fixtures/', import.meta.url);
  await page.locator('#source-file').setInputFiles({ name: 'session.larix', mimeType: 'application/octet-stream', buffer: readFileSync(new URL('valid/two-analyzed.larix', root)) });
  await expect(page.locator('#source-details')).toContainText('larix · 2 scans');
  await expect(page.locator('#source-scan option')).toHaveCount(2);
  await expect(page.locator('#reader-warnings')).toContainText('archival');
  const complex = page.locator('#dataset-paths option').filter({ hasText: /\/chir \[326\] · complex/ }).first();
  await expect(complex).toBeDisabled();
  await page.locator('#source-scan').selectOption('1');
  await expect(page.locator('#source-signal')).toHaveValue('0');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed session.larix.');
  const download = page.waitForEvent('download');
  await page.locator('#export-provenance').click();
  const record = JSON.parse(readFileSync((await (await download).path())!, 'utf8'));
  expect(record.input).toMatchObject({ format: 'larix', rows: 1420, selection: { scan: 1 } });
  await page.locator('#source-file').setInputFiles({ name: 'raw.larix', mimeType: 'application/octet-stream', buffer: readFileSync(new URL('valid/pf9a-raw.larix', root)) });
  await expect(page.locator('#source-scan option:checked')).toContainText('1426 points');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-error')).toContainText('strictly increasing');
  await page.locator('#source-file').setInputFiles({ name: 'broken.larix', mimeType: 'application/octet-stream', buffer: readFileSync(new URL('invalid/mismatched-energy-mu.larix', root)) });
  await expect(page.locator('#analysis-error')).toContainText('energy/mu');
  await expect(page.locator('#export-provenance')).toBeDisabled();
  await expect(page.locator('#process')).toBeDisabled();
  await page.locator('#source-file').setInputFiles({ name: 'recovered.larix', mimeType: 'application/octet-stream', buffer: readFileSync(new URL('valid/pfbl12c-raw.larix', root)) });
  await expect(page.locator('#source-scan option')).toHaveCount(1);
  await expect(page.locator('#analysis-error')).toBeHidden();
  await expect(page.locator('#custom-mapping')).not.toHaveAttribute('open', '');
  await expect(page.locator('#process')).toBeEnabled();
  await expect(page.locator('#export-provenance')).toBeDisabled();
});
