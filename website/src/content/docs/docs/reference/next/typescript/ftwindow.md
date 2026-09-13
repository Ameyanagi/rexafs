---
title: "TypeScript · FTWindow"
description: "FTWindow declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.5 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Supported Fourier-window families. Hanning and FHanning use cosine tapers; Parzen, Welch,
Gaussian, Sine and KaiserBessel use their named shapes. Forward and inverse settings default
to KaiserBessel; AUTOBK defaults to Hanning. Parameters dk/dr have shape-dependent meaning,
and KaiserBessel also uses them as shape parameters. See the [window
implementation](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/xafsutils.rs)
for the exact formulas.
Names are case-sensitive. An unsupported name throws an Error immediately during property
assignment, leaving the existing window selection unchanged.

```typescript
export type FTWindow = "Hanning" | "Parzen" | "Welch" | "Gaussian" | "Sine" | "KaiserBessel" | "FHanning";
```
