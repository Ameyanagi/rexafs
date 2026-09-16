---
title: "TypeScript · AUTOBKSolver"
description: "AUTOBKSolver declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.9.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Background spline solver. LinearDirect is the recommended default and is required for
FixedPenalty. LegacyLm is the iterative Levenberg-Marquardt solver for legacy objectives.
TrustRegionDogLeg requires a native Rust feature and is unavailable in the distributed Wasm
build; selecting it throws during processing.
An unrecognized name throws an Error immediately during property assignment and leaves the
existing selection unchanged.

```typescript
export type AUTOBKSolver = "TrustRegionDogLeg" | "LegacyLm" | "LinearDirect";
```
