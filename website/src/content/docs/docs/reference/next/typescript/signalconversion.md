---
title: "TypeScript · SignalConversion"
description: "SignalConversion declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.5 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Zero-based detector roles. Transmission is ln(incident/transmitted), with
nonzero matching polarity and matching units. Ratio is sum(detectors)/incident
with a nonzero monitor. No dark-current, gain or dead-time correction is inferred.
Direct copies the original signal and scale. See
[Newville (2014)](https://doi.org/10.2138/rmg.2014.78.2) for transmission assumptions.

```typescript
export type SignalConversion =
  | { kind: 'direct'; column: number }
  | { kind: 'transmission'; incident: number; transmitted: number }
  | { kind: 'ratio'; detectors: number[]; incident: number };
```
