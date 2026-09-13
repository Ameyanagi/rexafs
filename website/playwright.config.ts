import { defineConfig } from '@playwright/test';
export default defineConfig({
 testDir:'./tests/browser',timeout:30000,retries:0,use:{baseURL:'http://127.0.0.1:4321',headless:true},
 webServer:{command:'node scripts/preview-tests.mjs',url:'http://127.0.0.1:4321'+(process.env.SITE_BASE||'/rexafs').replace(/\/$/,'')+'/',reuseExistingServer:!process.env.CI},
});
