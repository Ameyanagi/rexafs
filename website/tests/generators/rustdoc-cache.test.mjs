import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, dirname } from 'node:path';
import { cacheKeys, replaceTree, restoreReference, saveReference } from '../../scripts/rustdoc-cache.mjs';

function fixture(t) {
  const root = mkdtempSync(resolve(tmpdir(), 'rexafs-rustdoc-cache-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (path, value) => {
    mkdirSync(dirname(resolve(root, path)), { recursive: true });
    writeFileSync(resolve(root, path), value);
  };
  execFileSync('git', ['init', '--quiet'], { cwd: root });
  for (const name of ['build-rust-reference.mjs', 'rustdoc-cache.mjs', 'rustdoc-header.html']) {
    write(`website/scripts/${name}`, readFileSync(resolve(import.meta.dirname, '../../scripts', name)));
  }
  write('Cargo.toml', '[workspace]\n');
  write('Cargo.lock', '# Locked dependencies\n');
  write('crates/rexafs/src/lib.rs', '/// A public API.\npub fn example() {}\n');
  write('crates/rexafs/src/data.json.gz', 'embedded data');
  const release = { version: '1.2.3', crateSha256: 'a'.repeat(64), verifiedAt: '2026-09-13' };
  const toolchain = [['rustc', 'pinned compiler'], ['rustdoc', 'pinned rustdoc'], ['cargo', 'pinned Cargo']];
  const keys = (metadata = release, compiler = toolchain, env = {}) => cacheKeys(root, metadata, compiler, env);
  return { root, write, release, toolchain, keys };
}

test('copy and release metadata changes reuse the relevant channel without hiding source changes', t => {
  const repo = fixture(t);
  const initial = repo.keys();
  repo.write('website/src/content/docs/index.md', 'Shorter homepage copy.');
  assert.deepEqual(repo.keys(), initial);
  assert.deepEqual(repo.keys({ ...repo.release, verifiedAt: '2026-09-14', downloads: ['new URL'] }), initial);
  const published = repo.keys({ ...repo.release, version: '1.2.4', crateSha256: 'b'.repeat(64) });
  assert.notEqual(published.stable, initial.stable);
  assert.equal(published.next, initial.next);
  repo.write('crates/rexafs/src/data.json.gz', 'changed embedded data');
  assert.equal(repo.keys().stable, initial.stable);
  assert.notEqual(repo.keys().next, initial.next);
  const beforeRemoval = repo.keys();
  rmSync(resolve(repo.root, 'crates/rexafs/src/lib.rs'));
  assert.notEqual(repo.keys().next, beforeRemoval.next);
});

test('headers, builder policy, compilers and effective build flags invalidate both channels', t => {
  const repo = fixture(t);
  const initial = repo.keys();
  const bothChanged = result => {
    assert.notEqual(result.stable, initial.stable);
    assert.notEqual(result.next, initial.next);
  };
  bothChanged(repo.keys(repo.release, [['rustc', 'new compiler']]));
  bothChanged(repo.keys(repo.release, repo.toolchain, { RUSTFLAGS: '-C target-cpu=native' }));
  bothChanged(repo.keys(repo.release, repo.toolchain, { CARGO_ENCODED_RUSTDOCFLAGS: '--document-private-items' }));
  for (const name of ['build-rust-reference.mjs', 'rustdoc-cache.mjs', 'rustdoc-header.html']) {
    const path = `website/scripts/${name}`;
    const original = readFileSync(resolve(repo.root, path));
    repo.write(path, Buffer.concat([original, Buffer.from('\n// Updated policy\n')]));
    bothChanged(repo.keys());
    repo.write(path, original);
  }
  repo.write('.cargo/config.toml', '[build]\ntarget = "aarch64-apple-darwin"\n');
  bothChanged(repo.keys());
});

test('locks and added local dependency inputs invalidate Next while Stable remains published', t => {
  const repo = fixture(t);
  for (const path of ['Cargo.lock', 'py-rexafs/Cargo.toml', 'crates/helper/build.rs', 'vendor/helper/input.txt']) {
    const before = repo.keys();
    repo.write(path, 'Changed dependency input');
    assert.notEqual(repo.keys().next, before.next);
    assert.equal(repo.keys().stable, before.stable);
  }
});

test('verified restoration removes obsolete pages and keeps Stable separate from Next', t => {
  const { root, write } = fixture(t);
  const source = resolve(root, 'generated');
  const stable = resolve(root, 'cache/stable');
  const next = resolve(root, 'cache/next');
  const output = resolve(root, 'public/rust');
  write('generated/rexafs/index.html', 'Stable API');
  write('generated/rexafs/old.html', 'Old API');
  saveReference(stable, 'stable-key', source);
  rmSync(resolve(source, 'rexafs/old.html'));
  write('generated/rexafs/index.html', 'Next API');
  saveReference(next, 'next-key', source);
  assert.equal(restoreReference(stable, 'stable-key', output), true);
  assert.equal(readFileSync(resolve(output, 'rexafs/index.html'), 'utf8'), 'Stable API');
  assert.equal(existsSync(resolve(output, 'rexafs/old.html')), true);
  assert.equal(restoreReference(next, 'next-key', output), true);
  assert.equal(readFileSync(resolve(output, 'rexafs/index.html'), 'utf8'), 'Next API');
  assert.equal(existsSync(resolve(output, 'rexafs/old.html')), false);
});

test('wrong identities, corruption, missing files and interrupted writes are cache misses', t => {
  const { root, write } = fixture(t);
  const source = resolve(root, 'generated');
  const cache = resolve(root, 'cache');
  const output = resolve(root, 'public/rust');
  write('generated/rexafs/index.html', 'Current API');
  write('generated/static.files/style.css', 'body {}');
  saveReference(cache, 'key', source);
  assert.equal(restoreReference(cache, 'different-source', output), false);
  assert.equal(existsSync(output), false);
  for (const mutate of [
    () => write('cache/html/rexafs/index.html', 'Corrupted API'),
    () => rmSync(resolve(cache, 'html/static.files/style.css')),
    () => write('cache/html/rexafs/obsolete.html', 'Unexpected page'),
    () => rmSync(resolve(cache, 'manifest.json')),
    () => symlinkSync(resolve(source, 'rexafs/index.html'), resolve(cache, 'html/rexafs/link.html')),
  ]) {
    saveReference(cache, 'key', source);
    mutate();
    assert.equal(restoreReference(cache, 'key', output), false);
    assert.equal(existsSync(output), false);
  }
  rmSync(resolve(source, 'rexafs/index.html'));
  assert.throws(() => saveReference(cache, 'key', source), /Missing Rust reference index/);
});

test('fresh builds replace generated output instead of merging old API pages', t => {
  const { root, write } = fixture(t);
  write('generated/rexafs/index.html', 'New API');
  write('public/rust/rexafs/removed.html', 'Removed API');
  replaceTree(resolve(root, 'generated'), resolve(root, 'public/rust'));
  assert.equal(existsSync(resolve(root, 'public/rust/rexafs/removed.html')), false);
  assert.equal(readFileSync(resolve(root, 'public/rust/rexafs/index.html'), 'utf8'), 'New API');
});
