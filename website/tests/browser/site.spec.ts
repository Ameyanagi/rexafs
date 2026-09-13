import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
const base=(process.env.SITE_BASE||'/rexafs').replace(/\/$/,'');
test('equal homepage entry points, accessible code tabs and responsive layout',async({page})=>{
 await page.goto(base+'/');
 await expect(page.getByRole('heading',{name:'Desktop',exact:true})).toBeVisible();
 await expect(page.getByRole('heading',{name:'Libraries',exact:true})).toBeVisible();
 await expect(page.getByRole('heading',{name:'Desktop',exact:true}).getByRole('link')).toHaveAttribute('href',base+'/docs/desktop/');
 await expect(page.getByRole('heading',{name:'Libraries',exact:true}).getByRole('link')).toHaveAttribute('href',base+'/docs/libraries/');
 await expect(page.getByRole('link',{name:'Start your first analysis with the rexafs desktop'})).toHaveAttribute('href',base+'/docs/getting-started/first-analysis/');
 for(const [name,path] of [['Python API →','docs/reference/stable/python/spectrum/'],['TypeScript API →','docs/reference/stable/typescript/spectrum/'],['Rust API →','api/rust/rexafs/index.html']]) {
  await expect(page.getByRole('link',{name,exact:true})).toHaveAttribute('href',base+'/'+path);
 }
 await page.getByRole('tab',{name:'TypeScript',exact:true}).click();
 await expect(page.getByRole('tabpanel')).toContainText('rexafs/node');
 await page.getByRole('tab',{name:'TypeScript',exact:true}).press('ArrowRight');
 await expect(page.getByRole('tab',{name:'Rust',exact:true})).toHaveAttribute('aria-selected','true');
 await page.screenshot({path:'test-results/home-desktop.png',fullPage:true});
 expect((await new AxeBuilder({page}).analyze()).violations).toEqual([]);
 await page.setViewportSize({width:390,height:844});
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBeTruthy();
 await expect(page.getByRole('link',{name:'Download the desktop',exact:true})).toBeVisible();
 await expect(page.getByRole('link',{name:'Use the libraries',exact:true})).toBeVisible();
 await page.screenshot({path:'test-results/home-mobile.png',fullPage:true});
});

test('Rust reference keeps navigation inside the content and supports narrow screens',async({page})=>{
 const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
 for(const colorScheme of ['light','dark'] as const) {
  await page.emulateMedia({colorScheme});
  await page.goto(base+'/api/rust/rexafs/index.html');
  const banner=page.getByRole('navigation',{name:'rexafs documentation'});
  await expect(banner).toBeVisible();
  await expect(banner.getByRole('link',{name:'← rexafs user guide'})).toHaveAttribute('href',base+'/docs/libraries/rust/');
  expect(await banner.evaluate(el=>Boolean(el.closest('main')))).toBeTruthy();
  expect(await page.locator('nav.sidebar').evaluate(el=>el.getBoundingClientRect().left)).toBe(0);
  expect(await banner.evaluate(el=>el.getBoundingClientRect().height)).toBeLessThan(100);
  await page.screenshot({path:`test-results/rust-${colorScheme}.png`,fullPage:true});
 }
 await page.goto(base+'/api/rust/rexafs/xafs/background/struct.AUTOBK.html');
 await expect(page.locator('[id="structfield.clamp_lambda"]')).toBeVisible();
 await page.setViewportSize({width:390,height:844});
 await expect(page.getByRole('navigation',{name:'rexafs documentation'})).toBeVisible();
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBeTruthy();
 await page.screenshot({path:'test-results/rust-mobile.png',fullPage:true});
 await page.goto(base+'/api/rust-next/rexafs/xafs/xasspectrum/struct.XASSpectrum.html');
 await expect(page.getByRole('navigation',{name:'rexafs documentation'})).toContainText('Next Rust API · unreleased');
 await expect(page.locator('[id="method.set_background_method"] .code-header')).toContainText('impl Into');
 expect(errors).toEqual([]);
});

test('released API pages explain settings, methods and units',async({page})=>{
 for(const language of ['python','typescript']) {
  await page.goto(base+`/docs/reference/stable/${language}/autobk/`);
  await expect(page.locator('main')).toContainText(/Default: 1\.0|default.*1\.0/i);
  await expect(page.locator('main')).toContainText(/low.R|low R/i);
  await expect(page.locator('main')).not.toContainText('Initialize self. See help(type(self))');
  await expect(page.locator('main a[href="https://doi.org/10.1103/PhysRevB.47.14126"]').first()).toBeVisible();
  await page.screenshot({path:`test-results/${language}-autobk.png`,fullPage:true});
  await page.goto(base+`/docs/reference/stable/${language}/spectrum/`);
  await expect(page.locator('main')).toContainText(/sqrt\(pi\)|√π|sqrt\(π\)/);
 }
});
test('science renders equations and search finds public content',async({page})=>{
 const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(base+'/docs/science/processing/');
 await expect(page.locator('.katex').first()).toBeVisible();
 await page.screenshot({path:'test-results/science.png',fullPage:true});
 await page.getByRole('button',{name:'Search',exact:true}).click();
 const search=page.getByRole('textbox',{name:'Search',exact:true});await search.fill('rbkg');
 await expect(page.locator('.pagefind-ui__result').first()).toBeVisible();
 await expect(page.locator('.pagefind-ui__results')).toContainText(/AUTOBK|Background/);
 await search.fill('documentation-site-plan');
 await expect(page.locator('.pagefind-ui__message')).toContainText('documentation-site-plan');
 for(const href of await page.locator('.pagefind-ui__result-link').evaluateAll(links=>links.map(a=>(a as HTMLAnchorElement).href))) {
  expect(href).not.toMatch(/documentation-site-plan|reference\/next|doc\/validation/);
 }
 await search.fill('zzzxrexafsunknownword');
 await expect(page.locator('.pagefind-ui__message')).toContainText(/No results/i);
 expect(errors).toEqual([]);
});
test('manual passes key accessibility checks on mobile and screenshots open at full resolution',async({page})=>{
 await page.setViewportSize({width:390,height:844});
 await page.goto(base+'/docs/getting-started/first-analysis/');
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBeTruthy();
 const image=page.locator('main img').first();await expect(image).toBeVisible();
 expect(await image.evaluate((i:HTMLImageElement)=>i.naturalWidth)).toBe(1192);
 expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa']).analyze()).violations).toEqual([]);
});

test('download links and manual light/dark themes remain usable',async({page})=>{
 await page.goto(base+'/download/');
 expect((await new AxeBuilder({page}).analyze()).violations).toEqual([]);
 await page.goto(base+'/docs/desktop/fitting/');
 for(const theme of ['light','dark']) {
  await page.getByRole('combobox',{name:'Select theme'}).selectOption(theme);
  expect((await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa']).analyze()).violations).toEqual([]);
  await page.screenshot({path:`test-results/fit-${theme}.png`,fullPage:true});
 }
});
