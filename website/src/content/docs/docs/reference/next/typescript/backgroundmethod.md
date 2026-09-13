---
title: "TypeScript · BackgroundMethod"
description: "BackgroundMethod declarations and JSDoc."
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

## AUTOBK

```typescript
static AUTOBK(parameters: AUTOBK): BackgroundMethod;
```

Copy AUTOBK settings into a background method.

## new_autobk

```typescript
static new_autobk(): BackgroundMethod;
```

Create the recommended default AUTOBK method.

## new_ilpbkg

```typescript
static new_ilpbkg(): BackgroundMethod;
```

Create an unimplemented ILPBkg placeholder; processing raises an error.
