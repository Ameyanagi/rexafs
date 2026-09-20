---
title: "TypeScript · MeasurementScan"
description: "MeasurementScan declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.12 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

One scan or project group; reading alone does not convert detector values.

## id

```typescript
id: string;
```

Original scan identifier or dataset-group path.

## label

```typescript
label: string;
```

Display label from the source.

## columns

```typescript
columns: MeasurementColumn[];
```

Channels in source order, retaining units and repeated energies.

## header

```typescript
header: string;
```

Original header, or complete XTUNES record text, retained for interpretation.

## metadata

```typescript
metadata: Record<string, string>;
```

Extracted metadata. XTUNES ordered_parameters holds ordered JSON section/key/value triples.

## signals

```typescript
signals: SignalCandidate[];
```

Detected choices; empty means that explicit mapping is required.

## warnings

```typescript
warnings: string[];
```

Unit assumptions, conflicting metadata and historical-format observations.
