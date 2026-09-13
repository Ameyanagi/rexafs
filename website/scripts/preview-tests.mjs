// Keep the preview in Playwright's process tree so it can be stopped reliably.
// Astro's interactive CLI can otherwise daemonize in an agent environment.
import { preview } from 'astro';
const server = await preview({ root: new URL('../', import.meta.url).pathname, server: { host: '127.0.0.1', port: 4321 } });
for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, async () => { await server.stop(); process.exit(0); });
