---
title: "TypeScript · FFTGrid"
description: "FFTGrid declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.9.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Forward-transform sampling convention. Input preserves the background k grid; Larch
constructs a zero-origin uniform grid with linear resampling and an extended window domain.
The default is Input. This choice affects Fourier preparation only, not the background
k()/chi() arrays.
Names are case-sensitive. Assigning an unsupported grid throws a string exception and
leaves the existing selection unchanged.

```typescript
export type FFTGrid = "Input" | "Larch";
```
