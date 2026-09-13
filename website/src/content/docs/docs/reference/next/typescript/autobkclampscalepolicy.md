---
title: "TypeScript · AUTOBKClampScalePolicy"
description: "AUTOBKClampScalePolicy declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout, including additions not available in npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Background endpoint model. FixedPenalty is the recommended fixed mean-square endpoint
penalty, controlled by clamp_lambda and solved by LinearDirect. Fixed and TwoPass retain
historical residual-dependent scaling behavior. Selecting a policy changes the optimization
objective, not merely the numerical solver.
Names are case-sensitive; an unsupported name throws an Error during property assignment
without changing the existing selection.

```typescript
export type AUTOBKClampScalePolicy = "FixedPenalty" | "Fixed" | "TwoPass";
```
