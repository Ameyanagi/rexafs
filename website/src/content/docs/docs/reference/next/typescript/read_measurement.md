---
title: "TypeScript · read_measurement"
description: "read_measurement declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.12 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Parse measurement content with the shared Rust reader (since 0.2.6).
Accepts UTF-8 text or Uint8Array, including Node Buffers and browser file bytes.
Returns owned Measurement storage; free it after copying the arrays you need.
Same limits/errors as Measurement. No filesystem/network access or processing
occurs. Example: read_measurement(new Uint8Array(await file.arrayBuffer())).

```typescript
export function read_measurement(data: string | Uint8Array): Measurement;
```
