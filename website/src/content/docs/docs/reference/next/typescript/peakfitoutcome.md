---
title: "TypeScript · PeakFitOutcome"
description: "PeakFitOutcome declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.11 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

One batch outcome, including failures. A nonconverged numerical result remains a result.

```typescript
export type PeakFitOutcome =
  | { index: number; result: PeakFitResult; error: null }
  | { index: number; result: null; error: string };
```
