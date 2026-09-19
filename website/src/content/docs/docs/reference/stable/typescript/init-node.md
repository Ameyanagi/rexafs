---
title: "TypeScript · init"
description: "init declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/node.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/node.d.ts)

Return an already-resolved initialization promise for Node.

Importing `rexafs/node` loads its packaged WebAssembly engine synchronously, so no explicit
initialization is required before constructing objects. This no-op export lets code shared
with browser callers use `await init()` in both places. A missing or invalid packaged Wasm
asset fails during module import instead.

```typescript
export default function init(): Promise<void>;
```
