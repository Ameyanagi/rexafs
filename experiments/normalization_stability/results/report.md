# Normalization stability experiment

The fit uses only the selected pre-edge and post-edge observations; the edge/XANES gap is excluded.
Each sweep is a 5 × 5 × 5 × 5 full-factorial variation of the four window endpoints (625 fits per method).

## Ru K edge (QAS)

Nominal/default-window edge steps:

| Method | Edge step | Derivative jump at E0 (μ/eV) |
|---|---:|---:|
| Independent Victoreen / quadratic | 0.862815 | -1.805e-04 |
| C1 anchored to pre-edge | 0.838708 | 0.000e+00 |
| C1 joint fit | 0.887229 | 0.000e+00 |
| Shared quadratic + step | 0.886212 | 0.000e+00 |

### Post-edge end varied from 300 to 940 eV

| Method | Step SD / median | Step 5–95% span | Norm RMS spread | Flat RMS spread | Pre RMSE / step | Post RMSE / step |
|---|---:|---:|---:|---:|---:|---:|
| Independent Victoreen / quadratic | 2.242% | 6.551% | 0.0234 | 0.0750 | 0.0029 | 0.0224 |
| C1 anchored to pre-edge | 1.826% | 6.012% | 0.0180 | 0.0566 | 0.0031 | 0.0356 |
| C1 joint fit | 3.716% | 10.131% | 0.0468 | 0.0690 | 0.0169 | 0.0206 |
| Shared quadratic + step | 3.611% | 9.807% | 0.0445 | 0.0560 | 0.0168 | 0.0205 |

### Post-edge end varied over 40–100% of available span

| Method | Step SD / median | Step 5–95% span | Norm RMS spread | Flat RMS spread | Pre RMSE / step | Post RMSE / step |
|---|---:|---:|---:|---:|---:|---:|
| Independent Victoreen / quadratic | 1.902% | 6.280% | 0.0204 | 0.0302 | 0.0029 | 0.0227 |
| C1 anchored to pre-edge | 1.624% | 5.392% | 0.0160 | 0.0366 | 0.0031 | 0.0402 |
| C1 joint fit | 3.179% | 9.747% | 0.0323 | 0.0293 | 0.0170 | 0.0211 |
| Shared quadratic + step | 3.221% | 9.881% | 0.0227 | 0.0267 | 0.0171 | 0.0209 |

## Cu K edge (150 K foil)

Nominal/default-window edge steps:

| Method | Edge step | Derivative jump at E0 (μ/eV) |
|---|---:|---:|
| Independent Victoreen / quadratic | 2.277425 | -6.808e-04 |
| C1 anchored to pre-edge | 2.043966 | 0.000e+00 |
| C1 joint fit | 2.435550 | 0.000e+00 |
| Shared quadratic + step | 2.373076 | 0.000e+00 |

### Post-edge end varied from 300 to 940 eV

| Method | Step SD / median | Step 5–95% span | Norm RMS spread | Flat RMS spread | Pre RMSE / step | Post RMSE / step |
|---|---:|---:|---:|---:|---:|---:|
| Independent Victoreen / quadratic | 7.726% | 13.267% | 0.0631 | 0.1764 | 0.0016 | 0.0446 |
| C1 anchored to pre-edge | 1.782% | 6.143% | 0.0164 | 0.0562 | 0.0017 | 0.0574 |
| C1 joint fit | 6.775% | 22.039% | 0.0759 | 0.1017 | 0.0237 | 0.0431 |
| Shared quadratic + step | 6.217% | 19.822% | 0.0699 | 0.0806 | 0.0206 | 0.0428 |

### Post-edge end varied over 40–100% of available span

| Method | Step SD / median | Step 5–95% span | Norm RMS spread | Flat RMS spread | Pre RMSE / step | Post RMSE / step |
|---|---:|---:|---:|---:|---:|---:|
| Independent Victoreen / quadratic | 1.282% | 4.000% | 0.0117 | 0.0053 | 0.0016 | 0.0340 |
| C1 anchored to pre-edge | 3.057% | 9.572% | 0.0302 | 0.0244 | 0.0018 | 0.1327 |
| C1 joint fit | 2.293% | 7.527% | 0.0094 | 0.0056 | 0.0293 | 0.0322 |
| Shared quadratic + step | 1.968% | 6.789% | 0.0064 | 0.0049 | 0.0237 | 0.0330 |

## Metric definitions

- **Step SD / median**: standard deviation of fitted edge steps divided by the median step.
- **Step 5–95% span**: central 90% edge-step range divided by the median step.
- **Norm/flat RMS spread**: RMS over energy of the pointwise standard deviation across all window selections.
- **Pre/post RMSE / step**: median common-window baseline residual divided by that run's edge step.
