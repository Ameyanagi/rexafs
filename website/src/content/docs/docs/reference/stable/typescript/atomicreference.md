---
title: "TypeScript · AtomicReference"
description: "AtomicReference declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Exact offline provider/table identity. Retain it for reproducible processing.

## data

```typescript
data: { provider: string; data_version: string; data_sha256: string };
```

Provider/profile version and actual decoded-data SHA-256.

## table

```typescript
table: "ChantlerF2LogLogV1" | "ElamTotalV1" | "ElamTransitionsV1";
```

Numerical table and interpolation/contribution profile.
