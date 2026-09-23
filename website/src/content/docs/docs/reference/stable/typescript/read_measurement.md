---
title: "TypeScript · read_measurement"
description: "read_measurement declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Parse measurement content with the shared Rust reader (since 0.2.6).
Accepts UTF-8 text or Uint8Array, including Node Buffers and browser file bytes.
Returns owned Measurement storage; free it after copying the arrays you need.
Same limits/errors as Measurement. No filesystem/network access or processing
occurs. Example: read_measurement(new Uint8Array(await file.arrayBuffer())).

```typescript
export function read_measurement(data: string | Uint8Array): Measurement;
```
