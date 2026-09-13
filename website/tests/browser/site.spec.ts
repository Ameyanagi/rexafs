import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
const base=(process.env.SITE_BASE||'/rexafs').replace(/\/$/,'');
test('equal homepage entry points, accessible code tabs and responsive layout',async({page})=>{
 await page.goto(base+'/');
 await expect(page.getByRole('heading',{name:'Desktop',exact:true})).toBeVisible();
 await expect(page.getByRole('heading',{name:'Libraries',exact:true})).toBeVisible();
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
