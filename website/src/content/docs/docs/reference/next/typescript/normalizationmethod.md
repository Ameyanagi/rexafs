---
title: "TypeScript · NormalizationMethod"
description: "NormalizationMethod declarations and JSDoc."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout, not npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

## constructor

```typescript
private constructor();
```

## free

```typescript
free(): void;
```

Release native memory. Do not use the object afterwards.

## PrePostEdge

```typescript
static PrePostEdge(parameters: PrePostEdge): NormalizationMethod;
```

Copy pre/post-edge settings into a normalization method.

## new_prepostedge

```typescript
static new_prepostedge(): NormalizationMethod;
```

Create automatic pre/post-edge normalization settings.

## new_mback

```typescript
static new_mback(): NormalizationMethod;
```

Create an unimplemented MBack placeholder; processing raises an error.
