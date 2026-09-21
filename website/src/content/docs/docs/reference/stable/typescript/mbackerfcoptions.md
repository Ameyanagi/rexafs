---
title: "TypeScript · MbackErfcOptions"
description: "MbackErfcOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.13.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Explicit bounds for the optional MBACK erfc background, not a sample correction.

## width

```typescript
width: [number, number];
```

Positive increasing width bounds in eV.

## amplitude

```typescript
amplitude: [number, number];
```

Increasing finite amplitude bounds in f2 units; signed values are allowed.

## family

```typescript
family?: boolean;
```

False selects an exact line such as Ka1; true selects a within-shell family such as Ka.
