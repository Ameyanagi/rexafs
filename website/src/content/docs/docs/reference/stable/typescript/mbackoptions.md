---
title: "TypeScript · MbackOptions"
description: "MbackOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Optional MBACK settings (since 0.2.10); energy and ranges use eV.

## e0

```typescript
e0?: number;
```

Fixed measured edge origin in eV; omitted uses derivative detection. Does not shift the table.

## pre_edge

```typescript
pre_edge?: [number, number];
```

Inclusive offsets from E0. Omitted suggests the outer 80% of the pre-edge span.

## post_edge

```typescript
post_edge?: [number, number];
```

Inclusive offsets from E0. Omitted suggests the outer 80% of the post-edge span.

## degree

```typescript
degree?: number;
```

Smooth-background polynomial degree 0–5, default 2. More flexibility can absorb real structure.

## erfc

```typescript
erfc?: MbackErfc;
```

Optional bounded fluorescence-background term; omitted disables erfc.
