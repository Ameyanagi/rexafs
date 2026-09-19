---
title: "TypeScript · PeakParameterOptions"
description: "PeakParameterOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Optional fixed values, bounds and restricted ties for an existing model parameter.

## vary

```typescript
vary?: boolean;
```

Independently vary this parameter, default true; a tie overrides this.

## bounds

```typescript
bounds?: readonly [number | null, number | null];
```

Inclusive minimum/maximum in parameter units; null means unbounded, the default.

## expression

```typescript
expression?: string;
```

Restricted expression in other parameter names; never external code.
