/** Build the checkout's browser engine and stage only its runtime assets for Astro. */
import { spawnSync, execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
const root = resolve(import.meta.dirname, '../..');
const target = resolve(root, 'website/public/wasm');
const { version, files } = JSON.parse(readFileSync(resolve(root, 'js-rexafs/package.json'), 'utf8'));
const build = spawnSync(process.execPath, ['js-rexafs/build.mjs'], { cwd: root, stdio: 'inherit' });
if (build.error) throw build.error;
if (build.status !== 0) process.exit(build.status ?? 1);
// Do not retain a module or helper from an older binding build.
rmSync(target, { recursive: true, force: true });
mkdirSync(resolve(target, 'dist/web'), { recursive: true });
// Use the package's runtime file list so new binding helpers are staged with
// their browser entry point. The Node entry point has a separate Wasm loader.
for (const file of files.filter(file => file.endsWith('.js') && file !== 'node.js')) {
  copyFileSync(resolve(root, 'js-rexafs', file), resolve(target, file));
}
for (const file of ['rexafs_wasm.js', 'rexafs_wasm_bg.wasm']) {
  copyFileSync(resolve(root, 'js-rexafs/dist/web', file), resolve(target, 'dist/web', file));
}
const manifest = {
  version,
  channel: 'source-preview',
  commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  dirty: Boolean(execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim()),
  wasmSha256: createHash('sha256').update(readFileSync(resolve(target, 'dist/web/rexafs_wasm_bg.wasm'))).digest('hex'),
};
writeFileSync(resolve(target, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
console.log(`Staged browser engine ${version} (${manifest.wasmSha256.slice(0, 12)})`);
