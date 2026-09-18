---
title: "TypeScript · NormalizationMethod"
description: "NormalizationMethod declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.10.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.10/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Select the normalization algorithm and hold an owned copy of its settings.

Use PrePostEdge(settings) for customized pre/post-edge fits or new_prepostedge() for
automatic defaults. The no-argument MBack factory is a historical empty selector and cannot
normalize data. MBACK was unimplemented through version 0.2.9. Copy this method into
Spectrum.set_normalization_method(), then free() the wrapper when no longer needed.

## free

```typescript
free(): void;
```

Release this object's Wasm allocation. Do not call methods, read fields or free it again
afterwards. Arrays and settings already copied elsewhere remain valid.

## PrePostEdge

```typescript
static PrePostEdge(parameters: PrePostEdge): NormalizationMethod;
```

Copy the supplied pre/post-edge configuration into a new owned algorithm wrapper. The input
settings are not consumed. Assign the wrapper to Spectrum.set_normalization_method(), then
free() it when no longer needed; the spectrum retains its own copy.

## new_prepostedge

```typescript
static new_prepostedge(): NormalizationMethod;
```

Create a new owned pre/post-edge normalization wrapper with automatic E0, fit ranges,
polynomial order and edge step. It does not process any spectrum. Assign it to
Spectrum.set_normalization_method() and free() the wrapper when finished.

## new_mback

```typescript
static new_mback(): NormalizationMethod;
```

Create the historical empty MBack selector. Selecting it makes normalize() throw.
Use new_prepostedge() for automatic polynomial normalization. MBACK was unimplemented
through version 0.2.9; this no-argument selector remains unusable for normalization.
Free this wrapper when finished.
