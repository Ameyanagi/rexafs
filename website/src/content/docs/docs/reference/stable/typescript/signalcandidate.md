---
title: "TypeScript · SignalCandidate"
description: "SignalCandidate declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

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
