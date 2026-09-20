---
title: "TypeScript · AbsorptionMode"
description: "AbsorptionMode declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Acquisition interpretation (since 0.2.10). Unknown means missing evidence, not
established fluorescence. Changing it never removes correction history.

```typescript
export type AbsorptionMode = "unknown" | "transmission" | "fluorescence";
```
