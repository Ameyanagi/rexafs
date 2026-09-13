---
title: "TypeScript · Spectrum"
description: "Spectrum declarations and JSDoc."
audience: user
pagefind: true
---


**Stable 0.2.4.** These signatures match the released npm package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

Mutable Rust spectrum. Stages run synchronously and return the same object.
Missing prerequisites use the selected algorithms and their defaults.
Array getters return independent copies, or undefined before their stage runs.

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/js-rexafs/types.d.ts)

## constructor

```typescript
constructor(energy: Float64Array, mu: Float64Array);
```

## from_arrays

```typescript
static from_arrays(energy: Float64Array, mu: Float64Array): Spectrum;
```

## free

```typescript
free(): void;
```

## set_spectrum

```typescript
set_spectrum(energy: Float64Array, mu: Float64Array): this;
```

## set_e0

```typescript
set_e0(e0: number): this;
```

## set_normalization_method

```typescript
set_normalization_method(method?: NormalizationMethod | null): this;
```

## set_background_method

```typescript
set_background_method(method?: BackgroundMethod | null): this;
```

## set_fft

```typescript
set_fft(parameters: XrayFFTF): this;
```

## e0

```typescript
e0(): number | undefined;
```

## find_e0

```typescript
find_e0(): this;
```

## normalize

```typescript
normalize(): this;
```

## calc_background

```typescript
calc_background(): this;
```

## fft

```typescript
fft(): this;
```

## ifft

```typescript
ifft(): this;
```

## invalidate_derived

```typescript
invalidate_derived(): this;
```

## k

```typescript
k(): Float64Array | undefined;
```

## chi

```typescript
chi(): Float64Array | undefined;
```

## norm

```typescript
norm(): Float64Array | undefined;
```

## flat

```typescript
flat(): Float64Array | undefined;
```

## pre_edge

```typescript
pre_edge(): Float64Array | undefined;
```

## post_edge

```typescript
post_edge(): Float64Array | undefined;
```

## r

```typescript
r(): Float64Array | undefined;
```

## kwin

```typescript
kwin(): Float64Array | undefined;
```

## kwin_k

```typescript
kwin_k(): Float64Array | undefined;
```

## chir_mag

```typescript
chir_mag(): Float64Array | undefined;
```

## chir_real

```typescript
chir_real(): Float64Array | undefined;
```

## chir_imag

```typescript
chir_imag(): Float64Array | undefined;
```

## q

```typescript
q(): Float64Array | undefined;
```

## chiq

```typescript
chiq(): Float64Array | undefined;
```
