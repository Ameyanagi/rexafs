import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { pinnedRelease, stageRefeff } from '../scripts/stage-refeff.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const python = process.env.REXAFS_PYTHON || 'python3';
const makeZip = String.raw`
import base64, json, stat, sys, warnings, zipfile
request = json.load(sys.stdin)
warnings.filterwarnings("ignore", category=UserWarning)
with zipfile.ZipFile(request["archive"], "w", zipfile.ZIP_DEFLATED) as archive:
    for name, content in request["entries"]:
        entry = zipfile.ZipInfo(name)
        if request.get("symlink") == name:
            entry.external_attr = (stat.S_IFLNK | 0o777) << 16
        archive.writestr(entry, base64.b64decode(content))
`;

async function fixture(t, options = {}) {
  const directory = await mkdtemp(resolve(tmpdir(), 'rexafs-refeff-stage-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const files = {
    'wasm/dist/index.mjs': 'export function runFeff() {}\n',
    'wasm/dist/worker.mjs': 'self.onmessage = () => {};\n',
    'wasm/dist/refeff.wasm': '\0asm\x01\0\0\0',
    'crates/refeff/tests/data/znse.inp': 'TITLE ZnSe\nEND\n',
    LICENSE: 'ReFEFF and FEFF10 license and port notices\n',
    'wasm/dist/browser-wasi-shim-LICENSE-MIT': 'MIT license fixture\n',
    'wasm/dist/browser-wasi-shim-LICENSE-APACHE': 'Apache license fixture\n',
    'wasm/README.md': '# Upstream browser integration fixture\n',
  };
  const info = {
    source_commit: options.commit ?? pinnedRelease.sourceCommit,
    refeff_version: pinnedRelease.version,
    cli_version: pinnedRelease.cliVersion,
    target: pinnedRelease.target,
    sha256: Object.fromEntries(Object.entries(files).map(([name, content]) => [name, hash(content)])),
  };
  if (options.badFileHash) info.sha256[options.badFileHash] = '0'.repeat(64);
  files['build-info.json'] = JSON.stringify(info);
  const prefix = `refeff-wasm-${pinnedRelease.version}/`;
  const entries = Object.entries(files).filter(([name]) => name !== options.omit)
    .map(([name, content]) => [prefix + name, Buffer.from(content).toString('base64')]);
  if (options.extra) entries.push([prefix + options.extra, Buffer.from('unexpected').toString('base64')]);
  if (options.duplicate) entries.push(entries[0]);
  const archive = resolve(directory, 'fixture.zip');
  const result = spawnSync(python, ['-c', makeZip], {
    input: JSON.stringify({ archive, entries, symlink: options.symlink ? prefix + options.symlink : undefined }),
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr || result.error?.message);
  const bytes = await readFile(archive);
  const release = {
    ...pinnedRelease,
    archiveSha256: hash(bytes),
    archiveBytes: bytes.length,
    wasmSha256: hash(files['wasm/dist/refeff.wasm']),
    wasmBytes: Buffer.byteLength(files['wasm/dist/refeff.wasm']),
  };
  // Bind the retained toolchain provenance to this tiny test module, without
  // requiring the 13 MB production binary for archive failure-path checks.
  const toolchainNoticesDirectory = resolve(directory, 'toolchain-notices');
  await mkdir(toolchainNoticesDirectory);
  release.toolchainNotices = { ...pinnedRelease.toolchainNotices };
  for (const name of Object.keys(pinnedRelease.toolchainNotices)) {
    const source = new URL(`../vendor/refeff-${pinnedRelease.version}/${name}`, import.meta.url);
    let data = await readFile(source);
    if (name === 'toolchain-provenance.json') {
      const provenance = JSON.parse(data);
      provenance.wasmSha256 = release.wasmSha256;
      data = Buffer.from(JSON.stringify(provenance));
      release.toolchainNotices[name] = { sha256: hash(data), bytes: data.length };
    }
    await writeFile(resolve(toolchainNoticesDirectory, name), data);
  }
  return {
    directory, archive, bytes, release,
    parameters: {
      release, archivePath: archive, outputDirectory: resolve(directory, 'public/refeff'),
      toolchainNoticesDirectory, python,
    },
  };
}

test('ReFEFF staging preserves upstream bytes and produces a reproducible offline cache', async t => {
  const { parameters, directory, bytes, release } = await fixture(t);
  const outputDirectory = parameters.outputDirectory;
  await mkdir(outputDirectory, { recursive: true });
  await writeFile(resolve(outputDirectory, 'old-version.mjs'), 'stale');
  const cacheDirectory = resolve(directory, 'cache');
  let fetches = 0;
  const settings = {
    ...parameters, archivePath: undefined, cacheDirectory,
    fetchArchive: async url => { assert.equal(url, release.archiveUrl); fetches++; return new Response(bytes); },
  };
  const manifest = await stageRefeff(settings);
  assert.equal(fetches, 1);
  assert.equal(manifest.sourceCommit, release.sourceCommit);
  assert.equal(manifest.wasmSha256, manifest.files['refeff.wasm'].sha256);
  assert.deepEqual(manifest.licenses, [
    'LICENSE', 'browser-wasi-shim-LICENSE-MIT', 'browser-wasi-shim-LICENSE-APACHE',
    'RUST-NOTICES.txt', 'TOOLCHAIN-NOTICES.txt', 'rust-COPYRIGHT-library.html',
  ]);
  assert.deepEqual(manifest.notices, ['NOTICE.md']);
  assert.deepEqual(manifest.provenance, ['upstream-build-info.json', 'rust-dependencies.json', 'toolchain-provenance.json']);
  assert.equal(manifest.files['NOTICE.md'].sourceUrl, release.notice.sourceUrl);
  assert.deepEqual((await readdir(outputDirectory)).sort(), [...Object.keys(manifest.files), 'manifest.json'].sort());
  for (const [name, file] of Object.entries(manifest.files)) {
    assert.equal(name.includes('/'), false, 'Runtime paths remain relative and flat for any website base');
    const data = await readFile(resolve(outputDirectory, name));
    assert.equal(hash(data), file.sha256);
    assert.equal(data.length, file.bytes);
  }
  const original = await readFile(resolve(outputDirectory, 'manifest.json'), 'utf8');
  await stageRefeff({ ...settings, fetchArchive: () => { throw new Error('Offline'); } });
  assert.equal(await readFile(resolve(outputDirectory, 'manifest.json'), 'utf8'), original);
  assert.equal(fetches, 1);
});

test('ReFEFF rejects corrupt cached bytes and preserves the previous public assets', async t => {
  const { parameters, archive } = await fixture(t);
  await stageRefeff(parameters);
  const original = await readFile(resolve(parameters.outputDirectory, 'manifest.json'), 'utf8');
  await writeFile(archive, 'corrupt');
  await assert.rejects(stageRefeff(parameters), /checksum or size mismatch/);
  assert.equal(await readFile(resolve(parameters.outputDirectory, 'manifest.json'), 'utf8'), original);
});

test('ReFEFF preserves the separately pinned source notice and rejects an altered copy', async t => {
  const { parameters, directory } = await fixture(t);
  await stageRefeff(parameters);
  const publicNotice = resolve(parameters.outputDirectory, 'NOTICE.md');
  const expected = await readFile(new URL('../third-party/refeff/NOTICE.md', import.meta.url));
  assert.deepEqual(await readFile(publicNotice), expected);
  assert.equal(hash(expected), pinnedRelease.notice.sha256);
  const noticePath = resolve(directory, 'altered-NOTICE.md');
  await writeFile(noticePath, 'Missing upstream attribution');
  await assert.rejects(stageRefeff({ ...parameters, noticePath }), /source notice checksum or size mismatch/);
  assert.deepEqual(await readFile(publicNotice), expected);
});

test('ReFEFF verifies the Rust notice bundle and its source inventory before replacing public assets', async t => {
  const { parameters, directory } = await fixture(t);
  await stageRefeff(parameters);
  const rustNoticesDirectory = resolve(directory, 'rust-notices');
  await mkdir(rustNoticesDirectory);
  const retained = {};
  for (const name of ['RUST-NOTICES.txt', 'rust-dependencies.json']) {
    const source = new URL(`../vendor/refeff-${pinnedRelease.version}/${name}`, import.meta.url);
    retained[name] = await readFile(source);
    assert.deepEqual(await readFile(resolve(parameters.outputDirectory, name)), retained[name]);
    await writeFile(resolve(rustNoticesDirectory, name), retained[name]);
  }
  for (const name of Object.keys(retained)) {
    await writeFile(resolve(rustNoticesDirectory, name), 'Incomplete dependency notices');
    await assert.rejects(stageRefeff({ ...parameters, rustNoticesDirectory }), /Rust notice checksum or size mismatch/);
    assert.deepEqual(await readFile(resolve(parameters.outputDirectory, name)), retained[name]);
    await writeFile(resolve(rustNoticesDirectory, name), retained[name]);
  }
  const inventory = JSON.parse(retained['rust-dependencies.json']);
  inventory.sourceCommit = '0'.repeat(40);
  const altered = JSON.stringify(inventory);
  await writeFile(resolve(rustNoticesDirectory, 'rust-dependencies.json'), altered);
  const release = {
    ...parameters.release,
    rustNotices: {
      ...parameters.release.rustNotices,
      'rust-dependencies.json': { sha256: hash(altered), bytes: Buffer.byteLength(altered) },
    },
  };
  await assert.rejects(stageRefeff({ ...parameters, release, rustNoticesDirectory }), /inventory does not match the pinned release/);
});

test('ReFEFF requires intact toolchain notices tied to the module, source and both notice payloads', async t => {
  const { parameters } = await fixture(t);
  await stageRefeff(parameters);
  const originalManifest = await readFile(resolve(parameters.outputDirectory, 'manifest.json'), 'utf8');
  const retained = {};
  for (const name of Object.keys(pinnedRelease.toolchainNotices)) {
    const path = resolve(parameters.toolchainNoticesDirectory, name);
    retained[name] = await readFile(path);
    assert.deepEqual(await readFile(resolve(parameters.outputDirectory, name)), retained[name]);
    await writeFile(path, 'Incomplete runtime notices');
    await assert.rejects(stageRefeff(parameters), /toolchain notice checksum or size mismatch/);
    await writeFile(path, retained[name]);
    assert.equal(await readFile(resolve(parameters.outputDirectory, 'manifest.json'), 'utf8'), originalManifest);
  }
  for (const alter of [
    record => { record.refeffSourceCommit = '0'.repeat(40); },
    record => { record.wasmSha256 = '0'.repeat(64); },
    record => { record.bundle.sha256 = '0'.repeat(64); },
    record => { record.rustStandardLibraryNotice.sha256 = '0'.repeat(64); },
  ]) {
    const provenance = JSON.parse(retained['toolchain-provenance.json']);
    alter(provenance);
    const data = JSON.stringify(provenance);
    await writeFile(resolve(parameters.toolchainNoticesDirectory, 'toolchain-provenance.json'), data);
    const release = {
      ...parameters.release,
      toolchainNotices: {
        ...parameters.release.toolchainNotices,
        'toolchain-provenance.json': { sha256: hash(data), bytes: Buffer.byteLength(data) },
      },
    };
    await assert.rejects(stageRefeff({ ...parameters, release }), /toolchain notice inventory does not match/);
    assert.equal(await readFile(resolve(parameters.outputDirectory, 'manifest.json'), 'utf8'), originalManifest);
  }
});

test('ReFEFF never caches a download with the wrong digest or excessive size', async t => {
  const { parameters, bytes, directory } = await fixture(t);
  const corrupt = Buffer.from(bytes);
  corrupt[0] ^= 1;
  for (const data of [corrupt, Buffer.concat([bytes, Buffer.from('extra')])]) {
    const settings = {
      ...parameters, archivePath: undefined, cacheDirectory: resolve(directory, 'cache'),
      fetchArchive: async () => new Response(data),
    };
    await assert.rejects(stageRefeff(settings), /checksum or size mismatch|exceeds the pinned size/);
    await assert.rejects(readdir(settings.cacheDirectory), { code: 'ENOENT' });
  }
});

test('ReFEFF requires runtime, example and license files even in a checksum-valid archive', async t => {
  for (const omit of ['wasm/dist/worker.mjs', 'crates/refeff/tests/data/znse.inp', 'LICENSE', 'wasm/dist/browser-wasi-shim-LICENSE-MIT']) {
    const { parameters } = await fixture(t, { omit });
    await assert.rejects(stageRefeff(parameters), /Missing ReFEFF archive file/);
    await assert.rejects(readdir(parameters.outputDirectory), { code: 'ENOENT' });
  }
});

test('ReFEFF checks embedded source identity and per-file digests', async t => {
  for (const options of [{ commit: '0'.repeat(40) }, { badFileHash: 'wasm/dist/worker.mjs' }]) {
    const { parameters } = await fixture(t, options);
    await assert.rejects(stageRefeff(parameters), /build identity mismatch|upstream file checksum mismatch/);
  }
  const { parameters } = await fixture(t);
  parameters.release.wasmSha256 = '0'.repeat(64);
  await assert.rejects(stageRefeff(parameters), /toolchain notice inventory does not match/);
});

test('ReFEFF refuses traversal, symlinks and duplicate ZIP members before extracting', async t => {
  for (const options of [{ extra: '../../escaped' }, { symlink: 'LICENSE' }, { duplicate: true }]) {
    const { parameters, directory } = await fixture(t, options);
    await assert.rejects(stageRefeff(parameters), /Unsafe ReFEFF archive member|Duplicate ReFEFF archive member/);
    await assert.rejects(readdir(parameters.outputDirectory), { code: 'ENOENT' });
    assert.deepEqual(await readdir(resolve(directory, 'public')), []);
  }
});
