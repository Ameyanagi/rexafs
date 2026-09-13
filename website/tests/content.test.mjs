import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve, relative } from 'node:path';
import { load } from 'cheerio';
import sharp from 'sharp';
const root=resolve(import.meta.dirname,'..');
const dist=resolve(root,'dist');
const base=(process.env.SITE_BASE||'/rexafs').replace(/\/$/,'');
const origin=process.env.SITE_URL||'https://ameyanagi.github.io';
const release=JSON.parse(readFileSync(resolve(root,'src/data/release.json'),'utf8'));
const releasedSource=path=>execFileSync('git',['show',`${release.tag}:${path}`],{cwd:resolve(root,'..'),encoding:'utf8'});
const releasedDeclarations={
 python:releasedSource('py-rexafs/python/rexafs/__init__.pyi'),
 typescript:releasedSource('js-rexafs/types.d.ts'),
};
const pythonConstructor=releasedDeclarations.python.match(/^class AUTOBK:[\s\S]*?^    def __init__\(([\s\S]*?)\) -> None:/m)?.[1];
assert(pythonConstructor,'released Python AUTOBK constructor must be present');
const pythonHasKeywordSettings=/(?:^|,)\s*\*(?:,|$)/.test(pythonConstructor);
const typescriptConstructor=releasedDeclarations.typescript.match(/^export class AUTOBK\s*\{[\s\S]*?^  (constructor\([^;]*\);)/m)?.[1];
assert(typescriptConstructor,'released TypeScript AUTOBK constructor must be present');
function files(dir){return readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?files(resolve(dir,e.name)):[resolve(dir,e.name)]);}
const pages=files(dist).filter(p=>p.endsWith('.html')&&!/\/api\/rust(?:-next)?\//.test(p));
const parsed=new Map();
const parse=file=>{if(!parsed.has(file))parsed.set(file,load(readFileSync(file,'utf8')));return parsed.get(file);};
test('public pages have working internal links, images and anchors at the deployment base',()=>{
 const failures=[];
 for(const file of pages){
  const $=parse(file); const current=new URL(base+'/'+relative(dist,file).replace(/index\.html$/,''),origin);
  $('[href], [src]').each((_,node)=>{
   const value=$(node).attr('href')??$(node).attr('src');if(!value||/^(data:|mailto:|javascript:)/.test(value))return;
   const target=new URL(value,current);if(target.origin!==current.origin)return;
   if(base && !target.pathname.startsWith(base+'/')) { failures.push(`${relative(dist,file)}: escapes site base: ${value}`);return; }
   let path=decodeURIComponent(target.pathname.slice(base.length));
   // Starlight emits its special /404/ route as 404.html for GitHub Pages.
   if(path==='/404/')path='/404.html';
   if(path.endsWith('/'))path+='index.html';
   let dest=resolve(dist,'.'+path);
   if(!existsSync(dest)){failures.push(`${relative(dist,file)}: missing ${value}`);return;}
   if(target.hash&&dest.endsWith('.html')&&!/\/api\/rust(?:-next)?\//.test(dest)){
    const id=decodeURIComponent(target.hash.slice(1));
    if(!parse(dest)('[id]').toArray().some(n=>parse(dest)(n).attr('id')===id))failures.push(`${relative(dist,file)}: missing anchor ${value}`);
   }
  });
 }
 assert.deepEqual([...new Set(failures)],[]);
});
test('only user documents enter the manual; next APIs remain labeled and out of default search',()=>{
 for(const path of files(resolve(root,'src/content/docs')).filter(p=>/\.mdx?$/.test(p))){
  const text=readFileSync(path,'utf8');assert.match(text,/^audience: user$/m,path);
  if(path.includes('/reference/next/'))assert.match(text,/^pagefind: false$/m,path);
 }
 const forbidden=['doc/validation','doc/benchmarks','experiments/normalization_stability','CONTRIBUTING.md','AGENTS.md','documentation-site-plan'];
 for(const file of files(dist))for(const value of forbidden)assert(!relative(dist,file).includes(value),file);
 for(const file of pages){const $=parse(file);const body=$('main').text();assert(!/Historical plan|Repository instructions|Release qualification|Unpublished qualification attempt/.test(body),file);}
});
test('scientific math, citations and full screenshots are present',async()=>{
 const theory=parse(resolve(dist,'docs/science/processing/index.html'));
 assert(theory('.katex').length>=20);assert.equal(theory('.katex-error').length,0);
 assert(theory('a[href="https://doi.org/10.1103/PhysRevB.47.14126"]').length);
 const refs=parse(resolve(dist,'docs/science/references/index.html'));
 assert(refs('a[href="https://doi.org/10.1103/RevModPhys.72.621"]').length);
 for(const name of ['welcome','import-mapping','normalize','background','transform','structure','paths','fit-ranges','fit-result','publication','save-project']){
  const data=await sharp(resolve(root,`public/screenshots/${name}.jpg`)).metadata();
  assert.equal(data.format,"jpeg");assert.equal(data.width,1192);assert.equal(data.height,768);
 }
});
test('published Rust docs and public API coverage are included',()=>{
 for(const path of ['index.html','xafs/analysis/index.html','xafs/fitting/index.html','xafs/structure/index.html','xafs/tools/index.html','plot/index.html'])assert(existsSync(resolve(dist,'api/rust/rexafs',path)),path);
 for(const channel of ['stable','next'])for(const language of ['python','typescript']){
  const path=resolve(dist,`docs/reference/${channel}/${language}/spectrum/index.html`);assert(existsSync(path));
  assert(parse(path)('main').text().includes('calc_background'));
 }
 const stable=parse(resolve(dist,'docs/reference/stable/python/autobk/index.html'))('main').text();
 if(pythonHasKeywordSettings)assert(stable.includes('AUTOBK(*,'),'released keyword constructor must be rendered');
 else assert(!stable.includes('AUTOBK(rbkg='),'unreleased keyword constructor must not be rendered');
 assert(parse(resolve(dist,'api/rust/rexafs/xafs/background/struct.AUTOBK.html'))('[id="structfield.clamp_lambda"]').length,'Rust reference must use the default numerical backend');
 assert(existsSync(resolve(dist,'api/rust-next/rexafs/xafs/xasspectrum/struct.XASSpectrum.html')));
});

test('generated API members have explanations and retain released signatures',()=>{
 const reference=resolve(root,'src/content/docs/docs/reference');
 for(const path of files(reference).filter(p=>p.endsWith('.md'))) {
  const text=readFileSync(path,'utf8');
  assert(!text.includes('Initialize self. See help(type(self))'),path);
  for(const section of text.split(/^## /m).slice(1)) {
   const afterSignature=section.replace(/^[\s\S]*?```[\s\S]*?```/,'').trim();
   assert(afterSignature.length>20,`${relative(root,path)}: undocumented ${section.split('\n')[0]}`);
  }
 }
 for(const language of ['python','typescript']) {
  const stable=readFileSync(resolve(reference,`stable/${language}/spectrum.md`),'utf8');
  const releasedInverse=language==='python'
   ? /^    def set_ifft\(/m.test(releasedDeclarations.python)
   : /^  set_ifft\(/m.test(releasedDeclarations.typescript);
  assert.equal(/^## set_ifft$/m.test(stable),releasedInverse,`${language}: stable inverse setter must match ${release.tag}`);
  const next=readFileSync(resolve(reference,`next/${language}/spectrum.md`),'utf8');
  assert(/^## set_ifft$/m.test(next));
  const releasedInverseSettings=language==='python'
   ? /^class XrayFFTR:/m.test(releasedDeclarations.python)
   : /^export class XrayFFTR\s*\{/m.test(releasedDeclarations.typescript);
  assert.equal(existsSync(resolve(reference,`stable/${language}/xrayfftr.md`)),releasedInverseSettings,`${language}: stable inverse settings must match ${release.tag}`);
  assert(existsSync(resolve(reference,`next/${language}/xrayfftr.md`)));
 }
 const py=readFileSync(resolve(reference,'stable/python/autobk.md'),'utf8');
 if(pythonHasKeywordSettings)assert.match(py,/```python\nAUTOBK\(\*, [^\n]*rbkg: float \| None=1\.0[^\n]*\)\n```/);
 else assert.match(py,/```python\nAUTOBK\(\)\n```/);
 const ts=readFileSync(resolve(reference,'stable/typescript/autobk.md'),'utf8');
 assert(ts.includes(`\`\`\`typescript\n${typescriptConstructor}\n\`\`\``),'stable TypeScript constructor must match the release declaration');
 assert(!readFileSync(resolve(reference,'stable/typescript/backgroundmethod.md'),'utf8').includes('private constructor'));
});
