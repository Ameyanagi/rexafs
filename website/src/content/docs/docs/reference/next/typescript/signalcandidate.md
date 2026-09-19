---
title: "TypeScript · SignalCandidate"
description: "SignalCandidate declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.11 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Header-supported signal choice; several choices require explicit selection.

## name

```typescript
name: string;
```

Display label for this signal.

## mapping

```typescript
mapping: SpectrumMapping<number>;
```

Fully specified axis and signal conversion.
