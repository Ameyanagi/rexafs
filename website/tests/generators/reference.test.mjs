import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, dirname } from 'node:path';
import { generate } from '../../scripts/generate-typescript-reference.mjs';

// Use a real release tag and edited checkout. This exercises the boundary that
// previously produced empty references: help must change without leaking new
// signatures into Stable, and missing source help must fail the build.
function fixture(t) {
  const root = mkdtempSync(resolve(tmpdir(), 'rexafs-reference-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (path, value) => {
    const target = resolve(root, path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, value);
  };
  const git = (...args) => execFileSync('git', args, { cwd: root, stdio: 'pipe' });
  write('website/src/data/release.json', JSON.stringify({ tag: 'v0.0.1' }));
  git('init', '--quiet');
  return {
    root, write,
    release() {
      git('add', '.');
      git('-c', 'user.name=Documentation test', '-c', 'user.email=docs@example.invalid',
        '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Release fixture');
      git('tag', 'v0.0.1');
    },
    page(channel, language, name) {
      return readFileSync(resolve(root,
        `website/src/content/docs/docs/reference/${channel}/${language}/${name}.md`), 'utf8');
    },
  };
}

test('TypeScript pages inherit edited source help while preserving released signatures', t => {
  const repo = fixture(t);
  const declaration = 'js-rexafs/types.d.ts';
  const before = `/** Configure the calculation. */
export class Settings {
  /** Create default settings. */
  constructor();
  /** Original field explanation. */
  scale: number;
}
`;
  repo.write(declaration, before);
  repo.write('js-rexafs/index.d.ts', '/** Initialize the browser runtime. */\nexport function init(): Promise<void>;');
  repo.write('js-rexafs/node.d.ts', '/** Initialize the Node runtime. */\nexport function init(): void;');
  repo.release();
  const updated = before.replace('Original field explanation.', 'Scale in inverse angstroms; default 2.')
    .replace('constructor();', 'constructor(scale?: number);\n  /** Clear computed values. */\n  reset(): void;');
  repo.write(declaration, updated);
  generate(repo.root);
  const stable = repo.page('stable', 'typescript', 'settings');
  const next = repo.page('next', 'typescript', 'settings');
  for (const page of [stable, next]) assert.match(page, /Scale in inverse angstroms; default 2\./);
  assert.match(stable, /constructor\(\);/);
  assert.doesNotMatch(stable, /## reset/);
  assert.match(next, /constructor\(scale\?: number\);/);
  assert.match(next, /## reset/);
  repo.write(declaration, updated.replace('/** Scale in inverse angstroms; default 2. */', ''));
  assert.throws(() => generate(repo.root), /Missing source JSDoc: Settings.scale/);
});

test('Python pages inherit class, attribute and function help and reject undocumented members', t => {
  const repo = fixture(t);
  const declaration = 'py-rexafs/python/rexafs/__init__.pyi';
  const before = `class Settings:
    """Configure the calculation."""
    def __init__(self) -> None:
        """Create default settings."""
    scale: float
    """Original field explanation."""
`;
  repo.write(declaration, before);
  repo.write('py-rexafs/python/rexafs/io.pyi', 'def read(path: str) -> Settings:\n    """Read a spectrum from a file."""\n');
  repo.release();
  const updated = before.replace('Original field explanation.', 'Scale in inverse angstroms; default 2.')
    .replace('__init__(self)', '__init__(self, *, scale: float = 2)')
    + '    def reset(self) -> None:\n        """Clear computed values."""\n';
  repo.write(declaration, updated);
  const run = () => execFileSync(process.env.PYTHON || 'python3', [
    resolve(import.meta.dirname, '../../scripts/generate-python-reference.py'), '--root', repo.root,
  ], { encoding: 'utf8', stdio: 'pipe' });
  run();
  const stable = repo.page('stable', 'python', 'settings');
  const next = repo.page('next', 'python', 'settings');
  for (const page of [stable, next]) {
    assert.match(page, /Configure the calculation\./);
    assert.match(page, /Scale in inverse angstroms; default 2\./);
  }
  assert.match(stable, /```python\nSettings\(\)\n```/);
  assert.doesNotMatch(stable, /## reset/);
  assert.match(next, /Settings\(\*, scale: float\s*=\s*2\)/);
  assert.match(next, /## reset/);
  assert.match(repo.page('stable', 'python', 'io-read'), /Read a spectrum from a file\./);
  repo.write(declaration, updated.replace('    """Scale in inverse angstroms; default 2."""\n', ''));
  assert.throws(run, /Missing source docstring: Settings.scale/);
});
