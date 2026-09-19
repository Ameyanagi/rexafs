---
title: "TypeScript · EnergyConversion"
description: "EnergyConversion declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.11 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Source axis conversion to eV; no magnitude-based unit guessing (since 0.2.6).

```typescript
export type EnergyConversion =
  | { kind: 'ev' }
  | { kind: 'kev' }
  | {
      kind: 'offset_ev';
      /** Finite origin in eV: absolute energy = source energy + offset_ev.
       * FDMNES detection uses its declared E_edge. The source axis stays unchanged. */
      offset_ev: number;
    }
  | {
      kind: 'bragg';
      /** Positive lattice-plane spacing in angstroms. Uses E = hc/(2 d sin(theta)). */
      d_spacing: number;
      /** Degrees per axis unit: 1 for degrees, 180/pi for radians. */
      degrees_per_unit: number;
    };
```
