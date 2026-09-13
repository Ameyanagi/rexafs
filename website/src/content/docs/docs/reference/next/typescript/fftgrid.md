---
title: "TypeScript · FFTGrid"
description: "FFTGrid declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** These signatures describe the source checkout, not npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Forward-transform sampling convention. Input preserves the background k grid; Larch
constructs a zero-origin uniform grid with linear resampling and an extended window domain.
The default is Input. This choice affects Fourier preparation only, not the background
k()/chi() arrays.

```typescript
export type FFTGrid = "Input" | "Larch";
```
