---
title: "TypeScript · AUTOBKSolver"
description: "AUTOBKSolver declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Background spline solver. LinearDirect is the recommended default and is required for
FixedPenalty. LegacyLm is the iterative Levenberg-Marquardt solver for legacy objectives.
TrustRegionDogLeg requires a native Rust feature and is unavailable in the distributed Wasm
build; selecting it throws during processing.
An unrecognized name throws an Error immediately during property assignment and leaves the
existing selection unchanged.

```typescript
export type AUTOBKSolver = "TrustRegionDogLeg" | "LegacyLm" | "LinearDirect";
```
