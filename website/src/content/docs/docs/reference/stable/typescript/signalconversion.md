---
title: "TypeScript · SignalConversion"
description: "SignalConversion declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Exact column names or zero-based detector roles. Transmission is ln(incident/transmitted), with
nonzero matching polarity and matching units. Ratio is sum(detectors)/incident
with a nonzero monitor. No dark-current, gain or dead-time correction is inferred.
Direct copies the original signal and scale. See
[Newville (2014)](https://doi.org/10.2138/rmg.2014.78.2) for transmission assumptions.

```typescript
export type SignalConversion<Column extends ColumnSelector = ColumnSelector> =
  | { kind: 'direct'; column: Column }
  | { kind: 'transmission'; incident: Column; transmitted: Column }
  | { kind: 'ratio'; detectors: Column[]; incident: Column };
```
