---
title: "TypeScript · AUTOBK"
description: "AUTOBK declarations and JSDoc."
audience: user
pagefind: true
---


**Stable 0.2.4.** These signatures match the released npm package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/js-rexafs/types.d.ts)

## constructor

```typescript
constructor();
```

## free

```typescript
free(): void;
```

## ek0

```typescript
ek0: number | undefined;
```

## rbkg

```typescript
rbkg: number | undefined;
```

## nknots

```typescript
nknots: number | undefined;
```

## kmin

```typescript
kmin: number | undefined;
```

## kmax

```typescript
kmax: number | undefined;
```

## kstep

```typescript
kstep: number | undefined;
```

## nclamp

```typescript
nclamp: number | undefined;
```

## clamp_lo

```typescript
clamp_lo: number | undefined;
```

## clamp_lambda

```typescript
clamp_lambda: number | undefined;
```

FixedPenalty strength; default 0.001, zero disables the endpoint penalty.

## clamp_hi

```typescript
clamp_hi: number | undefined;
```

## nfft

```typescript
nfft: number | undefined;
```

## kweight

```typescript
kweight: number | undefined;
```

## dk

```typescript
dk: number | undefined;
```

## linear_regularization

```typescript
linear_regularization: number | undefined;
```

## linear_condition_limit

```typescript
linear_condition_limit: number | undefined;
```

## linear_residual_ratio_limit

```typescript
linear_residual_ratio_limit: number | undefined;
```

## linear_fallback_to_lm

```typescript
linear_fallback_to_lm: boolean | undefined;
```

## linear_workspace_cache

```typescript
linear_workspace_cache: boolean | undefined;
```

## window

```typescript
window: FTWindow | undefined;
```

## solver

```typescript
solver: AUTOBKSolver | undefined;
```

## linear_fallback_solver

```typescript
linear_fallback_solver: AUTOBKSolver | undefined;
```

## clamp_scale_policy

```typescript
clamp_scale_policy: AUTOBKClampScalePolicy | undefined;
```
