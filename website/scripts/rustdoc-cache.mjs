/** Content identities and integrity checks for the two Rust reference caches. */
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, resolve } from 'node:path';

export const features = ['trust-region', 'plotting', 'refeff-runner', 'feff10-runner', 'amcsd', 'materials-project', 'cod'];
export const nextFlags = '-D rustdoc::broken_intra_doc_links -D missing_docs';
const digest = value => createHash('sha256').update(value).digest('hex');

function cargoConfiguration(root, environment) {
  const directories = [];
  for (let path = root; ; path = dirname(path)) {
    directories.push(resolve(path, '.cargo'));
    if (dirname(path) === path) break;
  }
  directories.push(resolve(environment.CARGO_HOME || resolve(homedir(), '.cargo')));
  return directories.flatMap((directory, index) => ['config', 'config.toml'].flatMap(name => {
    const path = resolve(directory, name);
    return existsSync(path) ? [[index, name, digest(readFileSync(path))]] : [];
  }));
}

/** Identify actual source bytes, including added files and embedded non-Rust data. */
function sourceInputs(root) {
  const paths = execFileSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard'], {
    cwd: root, encoding: 'utf8',
  }).split('\0').filter(Boolean).filter(path =>
    /^(Cargo\.(toml|lock)|rust-toolchain\.toml|py-rexafs\/Cargo\.toml)$/.test(path)
    || /^(\.cargo|crates|vendor)\//.test(path));
  return [...new Set(paths)].sort().map(path => [path,
    existsSync(resolve(root, path)) ? digest(readFileSync(resolve(root, path))) : null]);
}

/** Hash compiler identity, build policy and channel-specific source inputs.
 * Release download URLs and verification dates do not change Rust documentation.
 * Stable is identified by the published archive checksum; Next uses checkout bytes.
 */
export function cacheKeys(root, release, toolchain, environment = process.env) {
  const buildEnvironment = Object.entries(environment).filter(([name]) =>
    /^(RUST|CARGO_(BUILD|TARGET|PROFILE|ENCODED_RUST(?:DOC)?FLAGS)|CC($|_)|CXX($|_)|CFLAGS|CXXFLAGS|LDFLAGS|PKG_CONFIG)/.test(name)
    && name !== 'RUSTUP_HOME').sort(([a], [b]) => a.localeCompare(b));
  const shared = {
    schema: 1, toolchain, platform: process.platform, architecture: process.arch,
    features, buildEnvironment,
    scripts: ['build-rust-reference.mjs', 'rustdoc-cache.mjs', 'rustdoc-header.html'].map(name =>
      [name, digest(readFileSync(resolve(root, 'website/scripts', name)))]),
    cargoConfig: cargoConfiguration(root, environment),
  };
  return {
    stable: digest(JSON.stringify({ ...shared, version: release.version, crateSha256: release.crateSha256 })),
    next: digest(JSON.stringify({ ...shared, flags: nextFlags, sources: sourceInputs(root) })),
  };
}

/** Inventory every regular file; reject symlinks and incomplete documentation. */
function inventory(directory) {
  const files = [];
  function visit(relative = '') {
    for (const entry of readdirSync(resolve(directory, relative), { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const path = relative ? `${relative}/${entry.name}` : entry.name;
      if (entry.isDirectory()) visit(path);
      else if (entry.isFile()) files.push([path, digest(readFileSync(resolve(directory, path)))]);
      else throw Error(`Unexpected Rust reference cache entry: ${path}`);
    }
  }
  visit();
  if (!files.some(([path]) => path === 'rexafs/index.html')) throw Error('Missing Rust reference index');
  return files;
}

/** Replace a published channel completely, so removed pages cannot survive. */
export function replaceTree(source, destination) {
  rmSync(destination, { recursive: true, force: true });
  mkdirSync(dirname(destination), { recursive: true });
  cpSync(source, destination, { recursive: true });
}

/** Restore only matching, complete cache bytes; a miss leaves output untouched. */
export function restoreReference(cache, key, output) {
  try {
    const record = JSON.parse(readFileSync(resolve(cache, 'manifest.json'), 'utf8'));
    if (record.key !== key || JSON.stringify(record.files) !== JSON.stringify(inventory(resolve(cache, 'html')))) return false;
  } catch {
    return false;
  }
  replaceTree(resolve(cache, 'html'), output);
  return true;
}

/** Write the manifest last so interrupted builds cannot become cache hits. */
export function saveReference(cache, key, source) {
  const files = inventory(source);
  rmSync(cache, { recursive: true, force: true });
  replaceTree(source, resolve(cache, 'html'));
  writeFileSync(resolve(cache, 'manifest.json'), JSON.stringify({ key, files }) + '\n');
}
