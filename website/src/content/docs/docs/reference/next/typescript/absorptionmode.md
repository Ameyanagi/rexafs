---
title: "TypeScript · AbsorptionMode"
description: "AbsorptionMode declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.10 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Acquisition interpretation (since 0.2.10). Unknown means missing evidence, not
established fluorescence. Changing it never removes correction history.

```typescript
export type AbsorptionMode = "unknown" | "transmission" | "fluorescence";
```
