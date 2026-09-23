---
title: "TypeScript · FluorescenceCorrectionOptions"
description: "FluorescenceCorrectionOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Explicit sample geometry/emission plus optional internal-fit settings (since 0.2.10).

## line

```typescript
line: string;
```

Detected emission, for example Ka1; no line is inferred.

## angles

```typescript
angles: [number, number];
```

Measured [incidence, exit] angles in degrees FROM THE SAMPLE SURFACE, each in (0,90].

## family

```typescript
family?: boolean;
```

Default false selects one line; true selects an unresolved within-shell family, e.g. Ka.

## e0

```typescript
e0?: number;
```

Measured E0 in eV; omitted detects the edge. Does not shift the atomic table.

## pre_edge

```typescript
pre_edge?: [number, number];
```

Internal pre-edge eV offsets from E0; omitted uses available low endpoint to -30 eV.

## post_edge

```typescript
post_edge?: [number, number];
```

Internal post-edge eV offsets from E0; omitted uses +100 eV to available high endpoint.

## degree

```typescript
degree?: number;
```

Internal post-edge polynomial degree 0–5, default 1. Pre-edge is linear; no Victoreen term.
