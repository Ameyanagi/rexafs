import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const base = (process.env.SITE_BASE || '/rexafs').replace(/\/$/, '');
const fixtureRoot = new URL('../fixtures/refeff-0.4.0/', import.meta.url);
const referenceRecord = JSON.parse(readFileSync(new URL('provenance.json', fixtureRoot), 'utf8'));
const minimal = 'TITLE Browser input contract\nCONTROL 0 0 0 0 0 0\nPOTENTIALS\n0 29 Cu\nATOMS\n0 0 0 0 Cu\nEND\n';
const hash = (bytes: Uint8Array | string) => createHash('sha256').update(bytes).digest('hex');

async function download(page: Page, selector: string) {
  const pending = page.waitForEvent('download');
  await page.locator(selector).click();
  const item = await pending;
  expect(await item.failure()).toBeNull();
  return { name: item.suggestedFilename(), bytes: readFileSync((await item.path())!) };
}

async function record(page: Page) {
  return JSON.parse((await download(page, '#download-scattering-record')).bytes.toString('utf8'));
}

async function runMinimal(page: Page) {
  await page.locator('#feff-input').fill(minimal);
  await page.locator('#run-feff').click();
  await expect(page.locator('#download-scattering-record')).toBeEnabled({ timeout: 30_000 });
  const result = await record(page);
  expect(result.calculation).toMatchObject({
    exitCode: 0, report: { schema_version: 1, ok: true, data: { atoms: 1, effective_threads: 1 } },
  });
  await expect(page.locator('#scattering-error')).toBeHidden();
  return result;
}

function spectrumRows(bytes: Uint8Array) {
  return new TextDecoder().decode(bytes).split(/\r?\n/)
    .filter(line => line.trim() && !line.trimStart().startsWith('#'))
    .map(line => line.trim().split(/\s+/).map(Number));
}

// Expected arrays come from a separate, serial native executable. Its identity
// matches the published upstream release evidence; provenance.json records the
// exact input, executable and output hashes. Do not regenerate from the browser.
function compareNativeSpectrum(name: 'chi.dat' | 'xmu.dat', bytes: Uint8Array) {
  const native = readFileSync(new URL(name, fixtureRoot));
  expect(hash(native)).toBe(referenceRecord.outputs[name].sha256);
  const actual = spectrumRows(bytes), expected = spectrumRows(native);
  expect(actual.length).toBe(referenceRecord.outputs[name].numericRows);
  expect(actual.length).toBe(expected.length);
  let values = 0;
  for (let row = 0; row < expected.length; row++) {
    expect(actual[row].length).toBe(expected[row].length);
    for (let column = 0; column < expected[row].length; column++) {
      const a = actual[row][column], b = expected[row][column];
      expect(Number.isFinite(a) && Number.isFinite(b), `${name}[${row},${column}] is finite`).toBe(true);
      const tolerance = referenceRecord.comparison.absoluteTolerance
        + referenceRecord.comparison.relativeTolerance * Math.abs(b);
      expect(Math.abs(a - b), `${name}[${row},${column}]: ${a} versus native ${b}`).toBeLessThanOrEqual(tolerance);
      values++;
    }
  }
  return values;
}

test('released ReFEFF runs the complete ZnSe example locally and exports native-equivalent spectra', async ({ page }, testInfo) => {
  test.setTimeout(180_000);
  const errors: string[] = [], requests: { url: string; method: string }[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => requests.push({ url: request.url(), method: request.method() }));
  await page.goto(base + '/app/scattering/');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to calculation' })).toBeFocused();
  expect(await page.locator('.skip').evaluate(link => link.getBoundingClientRect().top)).toBeGreaterThanOrEqual(0);
  expect(requests.some(request => /\/refeff\/(?:refeff\.wasm|index\.mjs)/.test(request.url))).toBe(false);
  await page.locator('#load-znse').click();
  await expect(page.locator('#run-feff')).toBeEnabled();
  const input = await page.locator('#feff-input').inputValue();
  expect(hash(input)).toBe(referenceRecord.input.sha256);
  await page.evaluate(() => {
    const state = { ticks: 0, timer: 0 };
    state.timer = window.setInterval(() => {
      if (document.querySelector('#scattering-log')?.textContent?.trim()
        && !(document.querySelector('#cancel-feff') as HTMLButtonElement).disabled) state.ticks++;
    }, 10);
    Object.assign(window, { scatteringTestActivity: state });
  });
  await page.locator('#run-feff').click();
  await expect(page.locator('#download-scattering-record')).toBeEnabled({ timeout: 120_000 });
  const ticks = await page.evaluate(() => {
    const state = (window as unknown as { scatteringTestActivity: { ticks: number; timer: number } }).scatteringTestActivity;
    clearInterval(state.timer);
    return state.ticks;
  });
  expect(ticks).toBeGreaterThan(0);
  await expect(page.locator('#scattering-error')).toBeHidden();
  await expect(page.locator('#scattering-plot .data-line')).toHaveAttribute('points', /\d/);
  const provenance = await record(page);
  expect(provenance.schemaVersion).toBe(1);
  expect(provenance.calculation).toMatchObject({
    exitCode: 0, report: { schema_version: 1, ok: true, data: { atoms: 35, effective_threads: 1 } },
  });
  expect(provenance.calculation.report.data.stages.map((stage: { name: string }) => stage.name)).toEqual(
    expect.arrayContaining(['fms', 'ff2x']),
  );
  expect(Object.keys(provenance.outputs).filter(name => /^feff\d{4}\.dat$/.test(name))).toHaveLength(15);
  expect(provenance.inputs.root).toMatchObject({ name: 'feff.inp', text: input, sha256: hash(input), bytes: Buffer.byteLength(input), encoding: 'utf-8' });
  expect(provenance.inputs.auxiliaries).toEqual([]);
  expect(provenance.timing.elapsedSeconds).toBeGreaterThanOrEqual(0);
  expect(Number.isFinite(Date.parse(provenance.startedAt))).toBe(true);
  expect(Date.parse(provenance.completedAt)).toBeGreaterThanOrEqual(Date.parse(provenance.startedAt));

  const manifestResponse = await page.request.get(base + '/refeff/manifest.json');
  expect(manifestResponse.ok()).toBe(true);
  const engineManifest = await manifestResponse.json();
  expect(engineManifest).toMatchObject({ version: '0.4.0', sourceCommit: referenceRecord.refeff.sourceCommit });
  expect(provenance.engine).toMatchObject(engineManifest);
  const wasmResponse = await page.request.get(base + '/refeff/refeff.wasm');
  expect(wasmResponse.ok()).toBe(true);
  expect(JSON.stringify(provenance.engine)).toContain(hash(await wasmResponse.body()));
  expect(JSON.stringify(provenance.engine)).toContain(referenceRecord.refeff.sourceCommit);

  let compared = 0;
  for (const name of ['chi.dat', 'xmu.dat', 'pot.bin'] as const) {
    await page.locator('#output-file').selectOption(name);
    const file = await download(page, '#download-output');
    expect(file.name).toBe(name);
    expect(file.bytes.length).toBeGreaterThan(0);
    expect(provenance.outputs[name]).toEqual({ bytes: file.bytes.length, sha256: hash(file.bytes) });
    if (name !== 'pot.bin') compared += compareNativeSpectrum(name, file.bytes);
    const path = testInfo.outputPath(name);
    writeFileSync(path, file.bytes);
    await testInfo.attach(name, { path, contentType: name.endsWith('.dat') ? 'text/plain' : 'application/octet-stream' });
  }
  expect(compared).toBe(4010);
  const provenancePath = testInfo.outputPath('scattering-provenance.json');
  writeFileSync(provenancePath, JSON.stringify(provenance, null, 2) + '\n');
  await testInfo.attach('scattering-provenance.json', { path: provenancePath, contentType: 'application/json' });
  const skip = await page.locator('.skip').evaluate(link => ({ focused: document.activeElement === link, bottom: link.getBoundingClientRect().bottom }));
  expect(skip.focused).toBe(false);
  expect(skip.bottom).toBeLessThanOrEqual(0);
  await page.evaluate(() => new Promise<void>(resolve => { window.scrollTo(0, 0); requestAnimationFrame(() => resolve()); }));
  await page.screenshot({ path: testInfo.outputPath('scattering-desktop.png'), fullPage: true });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.evaluate(() => new Promise<void>(resolve => { window.scrollTo(0, 0); requestAnimationFrame(() => resolve()); }));
  await page.screenshot({ path: testInfo.outputPath('scattering-mobile.png'), fullPage: true });
  await page.locator('#feff-input').fill(input + '\n* Edited after this calculation.\n');
  await expect(page.locator('#scattering-stale')).toBeVisible();
  await expect(page.locator('#download-output')).toBeDisabled();
  await expect(page.locator('#download-scattering-record')).toBeDisabled();
  expect(errors).toEqual([]);
  expect(requests.filter(request => request.method !== 'GET')).toEqual([]);
  expect(requests.filter(request => !request.url.startsWith('blob:') && new URL(request.url).origin !== new URL(page.url()).origin)).toEqual([]);
});

test('local input files preserve auxiliary bytes and missing includes return a structured report', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto(base + '/app/scattering/');
  const input = 'INCLUDE model.inp\n';
  await page.locator('#feff-file').setInputFiles({ name: '銅.inp', mimeType: 'text/plain', buffer: Buffer.from(input) });
  await expect(page.locator('#feff-input')).toHaveValue(input);
  await expect(page.locator('#run-feff')).toBeEnabled();
  await page.getByText('Auxiliary files', { exact: true }).click();
  await page.locator('#aux-files').setInputFiles({ name: 'model.inp', mimeType: 'text/plain', buffer: Buffer.from(minimal) });
  await expect(page.locator('#clear-aux')).toBeEnabled();
  await page.locator('#run-feff').click();
  await expect(page.locator('#download-scattering-record')).toBeEnabled({ timeout: 30_000 });
  const first = await record(page);
  expect(first.calculation).toMatchObject({ exitCode: 0, report: { ok: true, data: { atoms: 1 } } });
  expect(first.inputs.root).toMatchObject({ sourceName: '銅.inp', text: input, sha256: hash(input) });
  expect(first.inputs.auxiliaries).toEqual([{
    name: 'model.inp', bytes: Buffer.byteLength(minimal), sha256: hash(minimal), encoding: 'base64', content: Buffer.from(minimal).toString('base64'),
  }]);
  await page.locator('#clear-aux').click();
  await expect(page.locator('#download-scattering-record')).toBeDisabled();
  await page.locator('#run-feff').click();
  await expect(page.locator('#scattering-error')).toContainText('model.inp', { timeout: 30_000 });
  const failed = await record(page);
  expect(failed.calculation.exitCode).not.toBe(0);
  expect(failed.calculation.report).toMatchObject({ schema_version: 1, ok: false, error: { message: expect.stringContaining('model.inp') } });
  expect(failed.inputs.auxiliaries).toEqual([]);
  await runMinimal(page);
});

test('cancelling engine loading discards the pending run and allows a fresh calculation', async ({ page }) => {
  test.setTimeout(60_000);
  let release!: () => void, requested!: () => void;
  const held = new Promise<void>(resolve => { release = resolve; });
  const started = new Promise<void>(resolve => { requested = resolve; });
  await page.route('**/refeff/refeff.wasm', async route => {
    requested();
    await held;
    await route.continue().catch(() => {});
  });
  try {
    await page.goto(base + '/app/scattering/');
    await page.locator('#feff-input').fill(minimal);
    await page.locator('#run-feff').click();
    await started;
    await page.locator('#cancel-feff').click();
    await expect(page.locator('#scattering-status')).toContainText(/cancelled/i);
    await expect(page.locator('#run-feff')).toBeEnabled();
    await expect(page.locator('#download-scattering-record')).toBeDisabled();
  } finally {
    release();
    await page.unroute('**/refeff/refeff.wasm');
  }
  await runMinimal(page);
});

test('cancelling a running Worker does not leak its outputs into the next calculation', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto(base + '/app/scattering/');
  await page.locator('#load-znse').click();
  await expect(page.locator('#run-feff')).toBeEnabled();
  // React to actual engine progress, avoiding an arbitrary delay or a race
  // between test-process scheduling and this small calculation finishing.
  await page.locator('#scattering-log').evaluate(log => {
    const observer = new MutationObserver(() => {
      if (!log.textContent?.trim()) return;
      observer.disconnect();
      (document.querySelector('#cancel-feff') as HTMLButtonElement).click();
    });
    observer.observe(log, { childList: true, subtree: true, characterData: true });
  });
  await page.locator('#run-feff').click();
  await expect(page.locator('#scattering-status')).toContainText(/cancelled/i, { timeout: 30_000 });
  await expect(page.locator('#download-output')).toBeDisabled();
  const next = await runMinimal(page);
  expect(next.inputs.root.text).toBe(minimal);
  expect(Object.keys(next.outputs).filter(name => /^feff\d{4}\.dat$/.test(name))).toEqual([]);
});

test('the five-minute budget includes engine loading and a timed-out calculation can retry', async ({ page }) => {
  test.setTimeout(60_000);
  await page.clock.install();
  let release!: () => void, requested!: () => void;
  const held = new Promise<void>(resolve => { release = resolve; });
  const started = new Promise<void>(resolve => { requested = resolve; });
  await page.route('**/refeff/refeff.wasm', async route => {
    requested();
    await held;
    await route.continue().catch(() => {});
  });
  try {
    await page.goto(base + '/app/scattering/');
    await page.locator('#feff-input').fill(minimal);
    await page.locator('#run-feff').click();
    await started;
    await page.clock.fastForward(300_001);
    await expect(page.locator('#scattering-error')).toContainText('5-minute');
    await expect(page.locator('#run-feff')).toBeEnabled();
    await expect(page.locator('#download-output')).toBeDisabled();
    await expect(page.locator('#download-scattering-record')).toBeDisabled();
  } finally {
    release();
    await page.unroute('**/refeff/refeff.wasm');
  }
  await runMinimal(page);
});

for (const failure of ['unavailable', 'altered'] as const) {
  test(`an ${failure} WASM asset fails visibly and a retry recovers`, async ({ page }) => {
    test.setTimeout(60_000);
    await page.route('**/refeff/refeff.wasm', route => route.fulfill(failure === 'unavailable'
      ? { status: 404, body: 'Missing test asset' }
      : { status: 200, contentType: 'application/wasm', body: Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]) }));
    await page.goto(base + '/app/scattering/');
    await page.locator('#feff-input').fill(minimal);
    await page.locator('#run-feff').click();
    await expect(page.locator('#scattering-error')).toContainText(failure === 'unavailable' ? /404/ : /SHA-256|hash|checksum/i);
    await expect(page.locator('#download-output')).toBeDisabled();
    await expect(page.locator('#download-scattering-record')).toBeDisabled();
    await expect(page.locator('#run-feff')).toBeEnabled();
    await page.unroute('**/refeff/refeff.wasm');
    await runMinimal(page);
  });
}
