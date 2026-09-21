import { defineConfig } from 'astro/config';
import { unified } from '@astrojs/markdown-remark';
import starlight from '@astrojs/starlight';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import baseLinks from './scripts/base-links.mjs';

const site = process.env.SITE_URL || 'https://ameyanagi.github.io';
const base = process.env.SITE_BASE || '/rexafs';

export default defineConfig({
  site, base, trailingSlash: 'always',
  markdown: { processor: unified({ remarkPlugins: [remarkMath, [baseLinks, { base }]], rehypePlugins: [[rehypeKatex, { strict: 'error', throwOnError: true }]] }) },
  integrations: [starlight({
    title: 'rexafs',
    description: 'X-ray absorption analysis, from your first spectrum to a reproducible fit. Desktop and libraries for Python, TypeScript and Rust.',
    favicon: '/favicon.svg',
    customCss: ['./src/styles/docs.css', 'katex/dist/katex.min.css'],
    components: { Header: './src/components/DocsHeader.astro' },
    editLink: { baseUrl: 'https://github.com/Ameyanagi/rexafs/edit/main/website/' },
    lastUpdated: true,
    social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/Ameyanagi/rexafs' }],
    sidebar: [
      { label: 'Start here', items: [{label:'Overview & features', slug:'docs/getting-started'}, {label:'Concepts & glossary', slug:'docs/concepts'}, {label:'Install rexafs', slug:'docs/getting-started/install'}, {label:'First analysis',slug:'docs/getting-started/first-analysis'}, {label:'Updates & offline use',slug:'docs/getting-started/updates'}] },
      { label: 'Desktop', items: [{label:'Desktop overview',slug:'docs/desktop'}, {label:'Import & groups',slug:'docs/desktop/import'}, {label:'XDI data',slug:'docs/desktop/xdi'}, {label:'Processing',slug:'docs/desktop/processing'}, {label:'Structures & paths',slug:'docs/desktop/structures'}, {label:'Fit a spectrum',slug:'docs/desktop/fitting'}, {label:'RMC refinement',slug:'docs/desktop/rmc'}, {label:'Multiple spectra & batches',slug:'docs/desktop/multiple-spectra'}, {label:'Series & trends',slug:'docs/desktop/series'}, {label:'Projects & recovery',slug:'docs/desktop/projects'}, {label:'Publication & exports',slug:'docs/desktop/publication'}, {label:'Optional assistant',slug:'docs/desktop/assistant'}] },
      { label: 'Libraries', items: [{label:'Choose a library',slug:'docs/libraries'}, {label:'Python',slug:'docs/libraries/python'}, {label:'TypeScript & JavaScript',slug:'docs/libraries/typescript'}, {label:'WebAssembly support',slug:'docs/libraries/webassembly'}, {label:'Rust',slug:'docs/libraries/rust'}, {label:'Spectrum API',slug:'docs/libraries/spectrum-api'}, {label:'API reference',slug:'docs/reference'}] },
      { label: 'Science', items: [{label:'Science overview',slug:'docs/science'}, {label:'How processing works',slug:'docs/science/processing'}, {label:'AUTOBK objective',slug:'docs/science/autobk'}, {label:'Fourier compatibility',slug:'docs/science/fourier-compatibility'}, {label:'Fit statistics',slug:'docs/science/fitting-statistics'}, {label:'LCF, PCA & data treatment',slug:'docs/science/analysis'}, {label:'References & citations',slug:'docs/science/references'}] },
      { label:'Help',items:[{label:'FAQ',slug:'docs/faq'},{label:'Troubleshooting',slug:'docs/troubleshooting'},{label:'Release history',slug:'releases'},{label:'Licenses & data',slug:'licenses'}]},
      { label: 'Support rexafs', items:[{label:'Sponsor development',link:'https://github.com/sponsors/Ameyanagi'},{label:'Star on GitHub',link:'https://github.com/Ameyanagi/rexafs'}] },
      { label: 'Stable API reference', collapsed:true, items:[{autogenerate:{directory:'docs/reference/stable',collapsed:true}}] },
      { label: 'Next tutorials · unreleased', collapsed: true, items: [{label:'Synthetic copper: PCA, MCR & LCF',slug:'docs/next/synthetic-copper'}] },
      { label: 'Next API · unreleased', collapsed:true, items:[{autogenerate:{directory:'docs/reference/next',collapsed:true}}] },
    ],
  })],
});
