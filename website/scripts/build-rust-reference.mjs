/** Build separately labeled published and source-checkout Rust references. */
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { appendFileSync, readFileSync, mkdirSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { cacheKeys, features, nextFlags, replaceTree, restoreReference, saveReference } from './rustdoc-cache.mjs';

const root = resolve(import.meta.dirname, '../..');
const release = JSON.parse(readFileSync(resolve(root, 'website/src/data/release.json'), 'utf8'));
const toolchain = ['rustc', 'rustdoc', 'cargo'].map(command => [command,
  execFileSync(command, ['--version', '--verbose'], { cwd: root, encoding: 'utf8' }).trim()]);
const keys = cacheKeys(root, release, toolchain);
const cache = resolve(process.env.DOCS_HTML_CACHE_DIR || resolve(root, 'website/.cache/rustdoc-html'));

if (process.argv.includes('--cache-keys')) {
  const text = `stable=${keys.stable}\nnext=${keys.next}\n`;
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, text);
  else process.stdout.write(text);
} else {
  const work = mkdtempSync(resolve(tmpdir(), 'rexafs-rustdoc-'));
  try {
    const target = resolve(process.env.DOCS_CARGO_TARGET_DIR || resolve(work, 'target'));
    // Keep the injected header path stable across dependency-cache restores.
    mkdirSync(target, { recursive: true });
    const template = readFileSync(resolve(root, 'website/scripts/rustdoc-header.html'), 'utf8');
    for (const channel of ['stable', 'next']) {
      const output = resolve(root, `website/public/api/${channel === 'stable' ? 'rust' : 'rust-next'}`);
      const channelCache = resolve(cache, channel);
      if (restoreReference(channelCache, keys[channel], output)) {
        console.log(`Restored verified ${channel} Rust reference (${keys[channel].slice(0, 12)})`);
        continue;
      }
      let manifest = resolve(root, 'crates/rexafs/Cargo.toml');
      if (channel === 'stable') {
        const archive = await fetch(`https://static.crates.io/crates/rexafs/rexafs-${release.version}.crate`);
        if (!archive.ok) throw Error(`Crate download failed: ${archive.status}`);
        const bytes = Buffer.from(await archive.arrayBuffer());
        if (createHash('sha256').update(bytes).digest('hex') !== release.crateSha256) throw Error('Published crate checksum mismatch');
        writeFileSync(resolve(work, 'source.crate'), bytes);
        execFileSync('tar', ['-xzf', resolve(work, 'source.crate'), '-C', work]);
        manifest = resolve(work, `rexafs-${release.version}/Cargo.toml`);
      }
      const header = resolve(target, `rustdoc-${channel}-header.html`);
      // Next's banner has no release version. Keep it independent of release metadata.
      writeFileSync(header, template.replaceAll('__REXAFS_VERSION__', channel === 'stable' ? release.version : 'unreleased'));
      // Cargo can retain pages for removed items or a different feature set.
      // Clear generated HTML before each miss, retaining compilation dependencies.
      rmSync(resolve(target, 'doc'), { recursive: true, force: true });
      execFileSync('cargo', ['doc', '--locked', '--manifest-path', manifest, '--no-deps', '--features', features.join(',')], {
        cwd: root, stdio: 'inherit',
        env: { ...process.env, CARGO_TARGET_DIR: target,
          // Encoded arguments also support header paths containing spaces and
          // enforce Next's checks when the caller has custom Rustdoc flags.
          CARGO_ENCODED_RUSTDOCFLAGS: ['--html-in-header', header,
            ...(channel === 'next' ? nextFlags.split(' ') : [])].join('\x1f') },
      });
      saveReference(channelCache, keys[channel], resolve(target, 'doc'));
      replaceTree(resolve(target, 'doc'), output);
      console.log(`Built ${channel} Rust reference (${keys[channel].slice(0, 12)})`);
    }
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}
