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
  expect(record.input.textSha256).toMatch(/^[a-f0-9]{64}$/);
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
  await page.locator('#signal-quantity').selectOption('transmission');
  await page.locator('#energy-unit').selectOption('keV');
  await expect(page.locator('#reference-column')).toHaveValue('1');
  await expect(page.locator('#signal-column')).toHaveValue('2');
  await page.locator('#process').click();
  await expect(page.locator('#analysis-status')).toHaveText('Processed transmission.csv.');
  await expect(page.locator('#result-e0')).toHaveText(`E₀ ${reference.e0.toFixed(2)} eV`);
  await page.locator('#source-file').setInputFiles({ name: 'duplicate.dat', mimeType: 'text/plain', buffer: Buffer.from('2 0.5\n2 0.6\n') });
  await expect(page.locator('#export-csv')).toBeDisabled();
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
