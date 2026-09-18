---
title: "TypeScript · MbackErfcOptions"
description: "MbackErfcOptions declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

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
