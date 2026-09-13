import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, relative } from 'node:path';
import { load } from 'cheerio';
import sharp from 'sharp';
const root=resolve(import.meta.dirname,'..');
const dist=resolve(root,'dist');
const base=(process.env.SITE_BASE||'/rexafs').replace(/\/$/,'');
const origin=process.env.SITE_URL||'https://ameyanagi.github.io';
function files(dir){return readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?files(resolve(dir,e.name)):[resolve(dir,e.name)]);}
const pages=files(dist).filter(p=>p.endsWith('.html')&&!p.includes('/api/rust/'));
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
   if(target.hash&&dest.endsWith('.html')&&!dest.includes('/api/rust/')){
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
 assert(!stable.includes('AUTOBK(rbkg='));
});
