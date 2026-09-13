/** Build rustdoc from the exact published crate, never an unreleased workspace. */
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
  const target=resolve(work,'target');
  execFileSync('cargo',['doc','--locked','--manifest-path',resolve(work,`rexafs-${release.version}/Cargo.toml`),'--no-deps','--all-features'],{stdio:'inherit',env:{...process.env,CARGO_TARGET_DIR:process.env.DOCS_CARGO_TARGET_DIR||target,RUSTDOCFLAGS:`--html-in-header ${resolve(root,'website/scripts/rustdoc-header.html')}`}});
  const output=resolve(root,'website/public/api/rust');
  rmSync(output,{recursive:true,force:true});mkdirSync(output,{recursive:true});
  cpSync(resolve(process.env.DOCS_CARGO_TARGET_DIR||target,'doc'),output,{recursive:true});
} finally {rmSync(work,{recursive:true,force:true});}
