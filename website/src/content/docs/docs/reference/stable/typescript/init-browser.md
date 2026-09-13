---
title: "TypeScript · init"
description: "init declarations and JSDoc."
audience: user
pagefind: true
---


**Stable 0.2.4.** These signatures match the released npm package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

Initialize Wasm. Node loads its local Wasm automatically; init is a no-op there.

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/js-rexafs/index.d.ts)

```typescript
export default function init(wasm?: URL | Request | Response | BufferSource | WebAssembly.Module): Promise<void>;
```
