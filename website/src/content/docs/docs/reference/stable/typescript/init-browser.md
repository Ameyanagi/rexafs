---
title: "TypeScript · init"
description: "init declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/js-rexafs/index.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/index.d.ts)

Load and initialize the browser WebAssembly engine before constructing spectra, settings or
algorithm wrappers. Await the returned promise before calling an API.

With no argument, the generated loader fetches the packaged `.wasm` file beside its
JavaScript glue module. Supply a URL, Request, Response, byte buffer or compiled
WebAssembly.Module when a bundler moves the asset. Fetch, compilation and instantiation
failures reject the promise. Calling init again after a successful initialization reuses the
initialized engine and ignores a different Wasm argument. Await one initialization before
starting processing; init() is not a way to replace an engine already in use.

The Node entry point loads its local Wasm automatically; init is a no-op there.

```typescript
export default function init(wasm?: URL | Request | Response | BufferSource | WebAssembly.Module): Promise<void>;
```
