/** Stage the checksum-pinned upstream ReFEFF browser release without rebuilding it. */
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const website = resolve(import.meta.dirname, '..');
export const pinnedRelease = JSON.parse(await readFile(resolve(website, 'src/data/refeff-release.json'), 'utf8'));
const licenses = [
  'LICENSE', 'browser-wasi-shim-LICENSE-MIT', 'browser-wasi-shim-LICENSE-APACHE',
  'RUST-NOTICES.txt', 'TOOLCHAIN-NOTICES.txt', 'rust-COPYRIGHT-library.html',
];
const rustNoticeNames = ['RUST-NOTICES.txt', 'rust-dependencies.json'];
const toolchainNoticeNames = ['TOOLCHAIN-NOTICES.txt', 'rust-COPYRIGHT-library.html', 'toolchain-provenance.json'];
const sources = {
  'index.mjs': 'wasm/dist/index.mjs',
  'worker.mjs': 'wasm/dist/worker.mjs',
  'refeff.wasm': 'wasm/dist/refeff.wasm',
  'znse.inp': 'crates/refeff/tests/data/znse.inp',
  LICENSE: 'LICENSE',
  'browser-wasi-shim-LICENSE-MIT': 'wasm/dist/browser-wasi-shim-LICENSE-MIT',
  'browser-wasi-shim-LICENSE-APACHE': 'wasm/dist/browser-wasi-shim-LICENSE-APACHE',
  'upstream-build-info.json': 'build-info.json',
  'upstream-README.md': 'wasm/README.md',
};
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

// Python's standard library handles ZIP decompression. Nothing is extracted by
// archive-provided pathname: only the fixed source-to-destination map is written.
const extract = String.raw`
import hashlib, json, pathlib, stat, sys, zipfile

request = json.load(sys.stdin)
release = request["release"]
root = "refeff-wasm-" + release["version"] + "/"
destination = pathlib.Path(request["destination"])
with zipfile.ZipFile(request["archive"]) as archive:
    names = set()
    total = 0
    for entry in archive.infolist():
        name = entry.orig_filename
        parts = name.rstrip("/").split("/")
        kind = stat.S_IFMT(entry.external_attr >> 16)
        if (name != entry.filename or "\\" in name or "\0" in name
                or any(part in ("", ".", "..") for part in parts)
                or not name.startswith(root)
                or kind not in (0, stat.S_IFREG, stat.S_IFDIR)):
            raise ValueError("Unsafe ReFEFF archive member: " + repr(name))
        if name in names:
            raise ValueError("Duplicate ReFEFF archive member: " + name)
        names.add(name)
        total += entry.file_size
        if entry.file_size > 20 * 1024 * 1024 or total > 40 * 1024 * 1024:
            raise ValueError("ReFEFF archive exceeds the extraction size limit")
    for source in request["sources"].values():
        if root + source not in names or archive.getinfo(root + source).is_dir():
            raise ValueError("Missing ReFEFF archive file: " + source)
    info = json.loads(archive.read(root + "build-info.json"))
    for key, expected in {
        "source_commit": release["sourceCommit"],
        "refeff_version": release["version"],
        "cli_version": release["cliVersion"],
        "target": release["target"],
    }.items():
        if info.get(key) != expected:
            raise ValueError("ReFEFF build identity mismatch: " + key)
    files = {}
    for name, source in request["sources"].items():
        data = archive.read(root + source)
        digest = hashlib.sha256(data).hexdigest()
        if source != "build-info.json" and info.get("sha256", {}).get(source) != digest:
            raise ValueError("ReFEFF upstream file checksum mismatch: " + source)
        if name == "refeff.wasm" and (
                digest != release["wasmSha256"] or len(data) != release["wasmBytes"]):
            raise ValueError("ReFEFF WebAssembly does not match the pinned release")
        (destination / name).write_bytes(data)
        files[name] = {"sha256": digest, "bytes": len(data), "sourcePath": source}
print(json.dumps(files))
`;

function verifyArchive(bytes, release, path) {
  if (bytes.length !== release.archiveBytes || sha256(bytes) !== release.archiveSha256) {
    throw new Error(`ReFEFF archive checksum or size mismatch: ${path}. Remove the invalid cache file before retrying.`);
  }
}

async function download(release, fetchArchive) {
  const response = await fetchArchive(release.archiveUrl, { signal: AbortSignal.timeout(60_000) });
  if (!response.ok || !response.body) throw new Error(`ReFEFF archive download failed: HTTP ${response.status}`);
  const chunks = [];
  let length = 0;
  for await (const chunk of response.body) {
    length += chunk.length;
    if (length > release.archiveBytes) throw new Error('ReFEFF archive download exceeds the pinned size');
    chunks.push(chunk);
  }
  const bytes = Buffer.concat(chunks);
  verifyArchive(bytes, release, release.archiveUrl);
  return bytes;
}

/**
 * Verify the release archive and its embedded source identity, then replace the
 * staged assets. An existing cache is rechecked on every call; a failed check
 * leaves existing public assets untouched. The manifest contains no timestamps
 * or absolute paths, so identical release bytes produce identical public files.
 * The archive omits upstream NOTICE.md; its separately pinned source copy is
 * retained alongside the ReFEFF/FEFF10 and browser WASI shim license texts.
 * A separately pinned inventory and notice bundle retain the selected Cargo
 * normal/build dependencies. Additional notices cover the identified Rust and
 * WASI runtime distribution. These tracked files need no Cargo or network fetch.
 * Python 3 is needed only during staging. The browser needs no Python runtime.
 */
export async function stageRefeff({
  release = pinnedRelease,
  archivePath,
  cacheDirectory = resolve(website, '.cache/refeff'),
  outputDirectory = resolve(website, 'public/refeff'),
  noticePath = resolve(website, 'third-party/refeff/NOTICE.md'),
  rustNoticesDirectory = resolve(website, `vendor/refeff-${release.version}`),
  toolchainNoticesDirectory = resolve(website, `vendor/refeff-${release.version}`),
  fetchArchive = fetch,
  python = process.env.REXAFS_PYTHON || 'python3',
} = {}) {
  if (!/^\d+\.\d+\.\d+$/.test(release.version) || release.tag !== `v${release.version}`
      || !/^[a-f0-9]{40}$/.test(release.sourceCommit)
      || !/^[a-f0-9]{64}$/.test(release.archiveSha256)
      || !/^[a-f0-9]{64}$/.test(release.wasmSha256)
      || !/^[a-f0-9]{64}$/.test(release.notice?.sha256)
      || release.notice.sourceUrl !== `https://raw.githubusercontent.com/Ameyanagi/refeff/${release.sourceCommit}/NOTICE.md`
      || !Number.isSafeInteger(release.notice.bytes) || release.notice.bytes <= 0
      || rustNoticeNames.some(name => !/^[a-f0-9]{64}$/.test(release.rustNotices?.[name]?.sha256)
        || !Number.isSafeInteger(release.rustNotices[name].bytes) || release.rustNotices[name].bytes <= 0)
      || toolchainNoticeNames.some(name => !/^[a-f0-9]{64}$/.test(release.toolchainNotices?.[name]?.sha256)
        || !Number.isSafeInteger(release.toolchainNotices[name].bytes) || release.toolchainNotices[name].bytes <= 0)
      || !Number.isSafeInteger(release.archiveBytes) || release.archiveBytes <= 0
      || release.archiveBytes > 20 * 1024 * 1024
      || !Number.isSafeInteger(release.wasmBytes) || release.wasmBytes <= 0
      || release.wasmBytes > 20 * 1024 * 1024) {
    throw new Error('Invalid pinned ReFEFF release metadata');
  }
  const notice = await readFile(noticePath);
  if (notice.length !== release.notice.bytes || sha256(notice) !== release.notice.sha256) {
    throw new Error(`ReFEFF source notice checksum or size mismatch: ${noticePath}`);
  }
  const rustNotices = {};
  for (const name of rustNoticeNames) {
    const data = await readFile(resolve(rustNoticesDirectory, name));
    const expected = release.rustNotices[name];
    if (data.length !== expected.bytes || sha256(data) !== expected.sha256) {
      throw new Error(`ReFEFF Rust notice checksum or size mismatch: ${name}`);
    }
    rustNotices[name] = data;
  }
  const inventory = JSON.parse(rustNotices['rust-dependencies.json']);
  if (inventory.schemaVersion !== 1 || inventory.refeffVersion !== release.version
      || inventory.sourceCommit !== release.sourceCommit || inventory.target !== release.target
      || inventory.rootPackage !== `refeff-cli@${release.cliVersion}`
      || inventory.bundle?.path !== 'RUST-NOTICES.txt'
      || inventory.bundle.sha256 !== release.rustNotices['RUST-NOTICES.txt'].sha256
      || inventory.bundle.bytes !== release.rustNotices['RUST-NOTICES.txt'].bytes
      || !Array.isArray(inventory.missingLicenseTexts) || inventory.missingLicenseTexts.length) {
    throw new Error('ReFEFF Rust notice inventory does not match the pinned release or bundle');
  }
  const toolchainNotices = {};
  for (const name of toolchainNoticeNames) {
    const data = await readFile(resolve(toolchainNoticesDirectory, name));
    const expected = release.toolchainNotices[name];
    if (data.length !== expected.bytes || sha256(data) !== expected.sha256) {
      throw new Error(`ReFEFF toolchain notice checksum or size mismatch: ${name}`);
    }
    toolchainNotices[name] = data;
  }
  const toolchain = JSON.parse(toolchainNotices['toolchain-provenance.json']);
  const matchesToolchainFile = (record, name) => record?.path === name
    && record.sha256 === release.toolchainNotices[name].sha256
    && record.bytes === release.toolchainNotices[name].bytes;
  if (toolchain.schemaVersion !== 1 || toolchain.refeffVersion !== release.version
      || toolchain.refeffSourceCommit !== release.sourceCommit || toolchain.target !== release.target
      || toolchain.wasmSha256 !== release.wasmSha256
      || !matchesToolchainFile(toolchain.bundle, 'TOOLCHAIN-NOTICES.txt')
      || !matchesToolchainFile(toolchain.rustStandardLibraryNotice, 'rust-COPYRIGHT-library.html')) {
    throw new Error('ReFEFF toolchain notice inventory does not match the pinned release or notice files');
  }
  const archive = archivePath ?? resolve(cacheDirectory, `refeff-${release.version}-${release.archiveSha256}.zip`);
  let bytes;
  try {
    bytes = await readFile(archive);
  } catch (error) {
    if (archivePath || error.code !== 'ENOENT') throw error;
    bytes = await download(release, fetchArchive);
    await mkdir(cacheDirectory, { recursive: true });
    const temporaryCache = await mkdtemp(resolve(cacheDirectory, '.download-'));
    try {
      const temporaryArchive = resolve(temporaryCache, 'archive.zip');
      await writeFile(temporaryArchive, bytes);
      await rename(temporaryArchive, archive);
    } finally {
      await rm(temporaryCache, { recursive: true, force: true });
    }
  }
  verifyArchive(bytes, release, archive);
  await mkdir(dirname(outputDirectory), { recursive: true });
  const temporaryOutput = await mkdtemp(resolve(dirname(outputDirectory), '.refeff-'));
  try {
    // Extract the exact bytes just verified, even if another process replaces
    // the shared cache while Python is starting.
    const verifiedArchive = resolve(temporaryOutput, '.verified-archive.zip');
    await writeFile(verifiedArchive, bytes);
    const extraction = spawnSync(python, ['-c', extract], {
      input: JSON.stringify({ archive: verifiedArchive, destination: temporaryOutput, release, sources }),
      encoding: 'utf8',
      maxBuffer: 1024 * 1024,
    });
    if (extraction.error) throw new Error(`Cannot run ${python} to stage ReFEFF: ${extraction.error.message}`);
    if (extraction.status !== 0) throw new Error(`Cannot stage ReFEFF: ${extraction.stderr.trim()}`);
    await rm(verifiedArchive);
    const files = JSON.parse(extraction.stdout);
    await writeFile(resolve(temporaryOutput, 'NOTICE.md'), notice);
    files['NOTICE.md'] = { ...release.notice, sourcePath: 'NOTICE.md' };
    for (const [name, data] of Object.entries(rustNotices)) {
      await writeFile(resolve(temporaryOutput, name), data);
      files[name] = {
        ...release.rustNotices[name],
        sourcePath: `website/vendor/refeff-${release.version}/${name}`,
        generatedBy: 'website/scripts/generate-refeff-notices.py',
      };
    }
    for (const [name, data] of Object.entries(toolchainNotices)) {
      await writeFile(resolve(temporaryOutput, name), data);
      files[name] = {
        ...release.toolchainNotices[name],
        sourcePath: `website/vendor/refeff-${release.version}/${name}`,
      };
    }
    const manifest = {
      ...release, files, licenses, notices: ['NOTICE.md'],
      provenance: ['upstream-build-info.json', 'rust-dependencies.json', 'toolchain-provenance.json'],
    };
    await writeFile(resolve(temporaryOutput, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
    await rm(outputDirectory, { recursive: true, force: true });
    await rename(temporaryOutput, outputDirectory);
    return manifest;
  } finally {
    await rm(temporaryOutput, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const manifest = await stageRefeff({ archivePath: process.env.REXAFS_REFEFF_ARCHIVE });
  console.log(`Staged ReFEFF ${manifest.version} (${manifest.wasmSha256.slice(0, 12)})`);
}
