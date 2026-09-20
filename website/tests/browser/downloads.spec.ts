import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';

const base = (process.env.SITE_BASE || '/rexafs').replace(/\/$/, '');
const release: {
  version: string;
  tag: string;
  platforms: { os: string; arch: string; url: string; sha256: string; note: string }[];
} = JSON.parse(readFileSync(new URL('../../src/data/release.json', import.meta.url), 'utf8'));
const operatingSystems = ['macOS', 'Windows', 'Linux'];
const isArm64 = (arch: string) => /arm64|aarch64|apple silicon/i.test(arch);

test('download inventory matches the release on desktop and mobile, including architecture requirements', async ({ page }) => {
  const [major, minor, patch] = release.version.split('.').map(Number);
  const windowsLinuxArm64 = major > 0 || minor > 2 || (minor === 2 && patch >= 5);
  const appleSiliconOnly = major > 0 || minor > 2 || (minor === 2 && patch >= 12);
  expect(release.platforms).toHaveLength(appleSiliconOnly ? 5 : windowsLinuxArm64 ? 6 : 4);
  expect(new Set(release.platforms.map(pkg => pkg.url)).size).toBe(release.platforms.length);
  expect(new Set(release.platforms.map(pkg => `${pkg.os}/${pkg.arch}`)).size).toBe(release.platforms.length);

  await page.goto(base + '/download/');
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 900 });
    const cards = page.locator('.downloads > article');
    await expect(cards).toHaveCount(3);
    await expect(cards.locator('h2')).toHaveText(operatingSystems);
    await expect(page.locator('.downloads a.download')).toHaveCount(release.platforms.length);
    await expect(page.locator('.downloads a.checksum')).toHaveCount(release.platforms.length);

    for (const [index, os] of operatingSystems.entries()) {
      const card = cards.nth(index);
      const packages = release.platforms.filter(pkg => pkg.os === os);
      if (os === 'macOS' && appleSiliconOnly) {
        expect(packages).toHaveLength(1);
        expect(packages[0].arch).toBe('Apple Silicon');
      } else if (windowsLinuxArm64 || os === 'macOS') {
        expect(packages).toHaveLength(2);
        expect(packages.filter(pkg => isArm64(pkg.arch))).toHaveLength(1);
      } else {
        expect(packages).toHaveLength(1);
      }
      const rows = card.locator('.architectures > li');
      await expect(rows).toHaveCount(packages.length);
      for (const [packageIndex, pkg] of packages.entries()) {
        const row = rows.nth(packageIndex);
        const download = row.locator('a.download');
        await expect(download).toHaveAttribute('href', pkg.url);
        await expect(row.locator('a.checksum')).toHaveAttribute('href', pkg.sha256);
        await expect(download).toBeVisible();
        await expect(row.locator('a.checksum')).toBeVisible();
        expect(pkg.url).toContain(`/releases/download/${release.tag}/rexafs-${release.version}-`);
        const architecture = isArm64(pkg.arch) ? 'aarch64' : 'x86_64';
        const suffix = os === 'macOS' ? 'apple-darwin.dmg'
          : os === 'Windows' ? 'pc-windows-msvc-setup.exe' : 'unknown-linux-gnu.tar.gz';
        expect(pkg.url).toBe(`https://github.com/Ameyanagi/rexafs/releases/download/${release.tag}/rexafs-${release.version}-${architecture}-${suffix}`);
        expect(pkg.sha256).toBe(`${pkg.url}.sha256`);

        const noteId = await download.getAttribute('aria-describedby');
        expect(noteId).toBeTruthy();
        const note = page.locator(`#${noteId}`);
        await expect(note).toHaveText(pkg.note);
        await expect(note).toBeVisible();
        if (os === 'Windows' && isArm64(pkg.arch)) {
          await expect(note).toContainText(/Windows 11/i);
          await expect(note).toContainText(/x64/i);
          await expect(note).toContainText(/FEFF10/i);
          await expect(note).toContainText(/helper|emulat/i);
        }
      }

      const hasArm64 = packages.some(pkg => isArm64(pkg.arch));
      if (windowsLinuxArm64) expect(hasArm64, `${os} must have an ARM64 package in ${release.version}`).toBe(true);
      if (hasArm64) {
        await expect(card.locator('.unavailable')).toHaveCount(0);
      } else {
        await expect(card.locator('.unavailable')).toContainText(`ARM64 · No desktop package in ${release.version}.`);
        await expect(card.locator('.unavailable')).toBeVisible();
        await expect(card.locator('.unavailable a')).toHaveAttribute('href', base + '/docs/getting-started/install/#arm64-availability');
      }
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
});

test('homepage code examples identify the release in the download metadata', async ({ page }) => {
  await page.goto(base + '/');
  await expect(page.locator('.code-note')).toHaveText(`Published API · ${release.version}`);
});
