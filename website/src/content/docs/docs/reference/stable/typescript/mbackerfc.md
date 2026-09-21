---
title: "TypeScript · MbackErfc"
description: "MbackErfc declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.13.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Immutable optional smooth-background term (since 0.2.10). It does not correct over-absorption.

## constructor

```typescript
constructor(line: string, options: MbackErfcOptions);
```

Select an emission originating at the absorber edge, with explicit scientific bounds.
