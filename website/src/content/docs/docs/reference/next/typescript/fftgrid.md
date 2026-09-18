---
title: "TypeScript · FFTGrid"
description: "FFTGrid declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.10 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Forward-transform sampling convention. Input preserves the background k grid; Larch
constructs a zero-origin uniform grid with linear resampling and an extended window domain.
The default is Input. This choice affects Fourier preparation only, not the background
k()/chi() arrays.
Names are case-sensitive. Assigning an unsupported grid throws a string exception and
leaves the existing selection unchanged.

```typescript
export type FFTGrid = "Input" | "Larch";
```
