---
title: "TypeScript · XrayFFTF"
description: "XrayFFTF declarations and JSDoc."
audience: user
pagefind: true
---


**Stable 0.2.4.** These signatures match the released npm package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/js-rexafs/types.d.ts)

## grid

```typescript
grid: FFTGrid;
```

Explicit sampling/window domain; defaults to Input.

## constructor

```typescript
constructor();
```

## free

```typescript
free(): void;
```

## rmax_out

```typescript
rmax_out: number | undefined;
```

## dk

```typescript
dk: number | undefined;
```

## dk2

```typescript
dk2: number | undefined;
```

## kmin

```typescript
kmin: number | undefined;
```

## kmax

```typescript
kmax: number | undefined;
```

## kweight

```typescript
kweight: number | undefined;
```

## nfft

```typescript
nfft: number | undefined;
```

## kstep

```typescript
kstep: number | undefined;
```

## window

```typescript
window: FTWindow | undefined;
```
