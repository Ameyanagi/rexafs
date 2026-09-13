/** Build separately labeled published and source-checkout Rust references. */
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, mkdirSync, writeFileSync, mkdtempSync, cpSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
const root=resolve(import.meta.dirname,'../..');
const release=JSON.parse(readFileSync(resolve(root,'website/src/data/release.json'),'utf8'));
const archive=await fetch(`https://static.crates.io/crates/rexafs/rexafs-${release.version}.crate`);
if(!archive.ok)throw Error(`Crate download failed: ${archive.status}`);
const bytes=Buffer.from(await archive.arrayBuffer());
if(createHash('sha256').update(bytes).digest('hex')!==release.crateSha256)throw Error('Published crate checksum mismatch');
const work=mkdtempSync(resolve(tmpdir(),'rexafs-rustdoc-'));
try {
  writeFileSync(resolve(work,'source.crate'),bytes);
  execFileSync('tar',['-xzf',resolve(work,'source.crate'),'-C',work]);
  const target=resolve(process.env.DOCS_CARGO_TARGET_DIR || resolve(work,'target'));
  // ndarray-compat replaces the default numerical modules with a legacy API.
  // Enable optional capabilities explicitly so the reference matches users'
  // default backend, including AUTOBK's fixed-penalty settings.
  const features = ['trust-region','plotting','refeff-runner','feff10-runner','amcsd','materials-project','cod'];
  const header = resolve(work, 'rustdoc-header.html');
  writeFileSync(header, readFileSync(resolve(root,'website/scripts/rustdoc-header.html'),'utf8').replaceAll('__REXAFS_VERSION__',release.version));
  // Keep compilation caches, but discard generated HTML before each channel.
  // Cargo can otherwise leave pages from removed or differently featured items
  // in the shared doc directory and accidentally publish them as released APIs.
  rmSync(resolve(target,'doc'),{recursive:true,force:true});
  execFileSync('cargo',['doc','--locked','--manifest-path',resolve(work,`rexafs-${release.version}/Cargo.toml`),'--no-deps','--features',features.join(',')],{stdio:'inherit',env:{...process.env,CARGO_TARGET_DIR:target,RUSTDOCFLAGS:`--html-in-header ${header}`}});
  const output=resolve(root,'website/public/api/rust');
  rmSync(output,{recursive:true,force:true});mkdirSync(output,{recursive:true});
  cpSync(resolve(target,'doc'),output,{recursive:true});
  // The checkout has newer docs and calling conventions. Publish it separately;
  // never replace stable signatures with the unreleased API.
  const nextHeader = resolve(work, 'rustdoc-next-header.html');
  writeFileSync(nextHeader, readFileSync(header, 'utf8'));
  rmSync(resolve(target,'doc'),{recursive:true,force:true});
  execFileSync('cargo',['doc','--locked','--manifest-path',resolve(root,'crates/rexafs/Cargo.toml'),'--no-deps','--features',features.join(',')],{stdio:'inherit',env:{...process.env,CARGO_TARGET_DIR:target,RUSTDOCFLAGS:`--html-in-header ${nextHeader} -D rustdoc::broken_intra_doc_links -D missing_docs`}});
  const nextOutput = resolve(root,'website/public/api/rust-next');
  rmSync(nextOutput,{recursive:true,force:true});mkdirSync(nextOutput,{recursive:true});
  cpSync(resolve(target,'doc'),nextOutput,{recursive:true});
} finally {rmSync(work,{recursive:true,force:true});}
