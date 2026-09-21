---
title: "TypeScript · BackgroundMethod"
description: "BackgroundMethod declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.13.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Select the background-removal algorithm and hold an owned copy of its settings.

Use AUTOBK(settings) for a configured spline background or new_autobk() for the recommended
defaults. The ILPBkg factory is only a placeholder; it does not implement that algorithm.
Copy this method into Spectrum.set_background_method(), then free() the wrapper when no
longer needed.

## free

```typescript
free(): void;
```

Release this object's Wasm allocation. Do not call methods, read fields or free it again
afterwards. Arrays and settings already copied elsewhere remain valid.

## AUTOBK

```typescript
static AUTOBK(parameters: AUTOBK): BackgroundMethod;
```

Copy the supplied AUTOBK configuration into a new owned algorithm wrapper. The input
settings are not consumed. Assign the wrapper to Spectrum.set_background_method(), then
free() it when no longer needed; the spectrum retains its own copy.

## new_autobk

```typescript
static new_autobk(): BackgroundMethod;
```

Create a new owned AUTOBK wrapper with the recommended LinearDirect/FixedPenalty defaults,
rbkg=1 angstrom and clamp_lambda=0.001. It does not process any spectrum. Assign it to
Spectrum.set_background_method() and free() the wrapper when finished.

## new_ilpbkg

```typescript
static new_ilpbkg(): BackgroundMethod;
```

Create an owned ILPBkg placeholder for API compatibility. This background method is not
implemented and calc_background() throws if selected. Use new_autobk() for supported
background removal, and free() any placeholder you create.
