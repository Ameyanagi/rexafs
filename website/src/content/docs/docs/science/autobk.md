---
title: "The AUTOBK objective"
description: "Understand low-R background removal and the fixed endpoint penalty."
audience: user
---

AUTOBK estimates a smooth absorption background by suppressing low-distance
Fourier components of the remaining EXAFS signal. The original method is
described by [Newville et al., *Physical Review B* 47, 14126–14131 (1993)](https://doi.org/10.1103/PhysRevB.47.14126).
The fixed endpoint penalty below is a **rexafs-specific modification**, not an
objective attributed to that paper. Start with the
[processing theory guide](/docs/science/processing/) for the absorption, k and R definitions.

New analyses in the default Rust backend, desktop, Python and Wasm use `LinearDirect` with `clamp_scale_policy = FixedPenalty` and
`clamp_lambda = 0.001`. This implements the weak fixed penalty tested in the
[clamp study](https://github.com/Ameyanagi/rexafs/blob/6cb668dfcba41f02db102fde7c8a091947468f83/doc/benchmarks/2026-09-07-clamp-study/README.md). The study's training
minimum was λ=0, with weak penalties practically tied; 0.001 is a conservative
weak-penalty choice, not a universally optimal value.

The optional Rust `ndarray-compat` feature retains the historical `Fixed` and
`TwoPass` objectives and does not expose `FixedPenalty` or `clamp_lambda`.
Do not enable that feature when reproducing the fixed-penalty method below.

For the vector of fitted cubic-spline coefficients **c**, let χ(c) be the edge-step-normalized spectrum,
minus an optional normalized χ standard **for the fitting objective only**.
Let **h(c)** contain the real and imaginary low-R Fourier residuals. The spline is linear in **c**. Its normalized residual χ(c) is therefore an
affine function of **c** when the energy grid, edge step, window and other
settings are fixed. The low-R residual vector **h(c)** contains the transformed
mismatch that should be small if the background has been removed. The objective is

```text
J(c) = ||h(c)||² / m
       + λ × [w_lo² Σ χ(first n points)² + w_hi² Σ χ(last n points)²] / N_active

m        = number of real/imaginary low-R residual entries
n        = min(nclamp, number of χ(k) points)
N_active = n × (number of nonzero endpoint weights)
```

Here, `||h||²` is the sum of squared entries, and each endpoint sum uses the
corresponding elements of χ(c). λ is `clamp_lambda`; `w_lo` and `w_hi` are the
absolute endpoint weights. The division by `m` makes the first term a mean
squared residual, and the division by `N_active` averages over enabled endpoint
samples. These are numerical residual conventions with the Fourier amplitude
scale specified below; λ is not a physical material property.

A zero weight excludes that end from both the sum and its denominator. If both
weights are zero, `nclamp = 0`, or `clamp_lambda = 0`, the endpoint term is absent.
The weights are the absolute values of `clamp_lo` and `clamp_hi`.

Defaults are `nclamp = 3`, `clamp_lo = 0`, and `clamp_hi = 1`: no low-end penalty,
and λ times the mean χ² of the final three points, including the last point.
At kstep=0.05 and kmax=12 these are k=11.90, 11.95, and 12.00 Å⁻¹. If enabled,
the first three points are k=0, 0.05, and 0.10 Å⁻¹. These are endpoints of the
output k grid; kmin controls the transform window and spline domain.

Doubling λ doubles the endpoint contribution to the objective. Doubling an
endpoint weight multiplies that end's contribution by four. Enabling both ends
shares the total penalty across twice as many endpoint points. There is no
initial-residual scale, update during a fit, second pass, or nonlinear fallback.

The low-R transform uses the k-weight and window selected for AUTOBK, with the
reference FFT amplitude factor `0.05 / sqrt(pi)` used by
[Larch's AUTOBK residual](https://github.com/xraypy/xraylarch/blob/860d8a690c81eefb0e61dee4ca3703ef4b67e93d/larch/xafs/autobk.py#L26).
This fixed **internal objective scale** differs from the public forward
transform, whose multiplier is the actual `kstep / sqrt(pi)`. At a nondefault
kstep, do not substitute the public FFT amplitude into this objective without
also accounting for the change in the relative endpoint penalty. The selected
kstep and nfft still determine the physical R grid and cutoff. The fixed
reference amplitude also makes the reported λ directly comparable with the
prototype at both tested k steps; changing the window or k-weight changes the
objective and can change the effective balance with the endpoint term.


## Spline flexibility and the low-R region

The spline is cubic in k, not in photon energy. More coefficients make it more
flexible. The default backend first computes an automatic count

$$
a=\left\lfloor\frac{2\,r_{\mathrm{bkg}}\,\Delta k}{\pi}\right\rfloor,
\qquad n_{\mathrm{auto}}=1+a,
\qquad \Delta k=k_{\max}-k_{\min}.
$$

Here $r_{\mathrm{bkg}}$ is `rbkg` in Å, $\Delta k$ is the positive fit span in
Å⁻¹, and $a$ and $n_{\mathrm{auto}}$ are dimensionless integers. The fitted
coefficient count is `nknots` when explicitly supplied, otherwise
$n_{\mathrm{auto}}$, bounded to 5–128. The samples used to initialize the spline
are chosen near evenly spaced k targets within this interval. These rules
limit flexibility; they do not prove that the fitted background is physically
unique.

The exact cutoff is an implementation convention rather than simply selecting
every R sample below the requested `rbkg`. Let
$\delta R=\pi/(\mathtt{nfft}\,\mathtt{kstep})$ in Å. For the cutoff calculation
only, set $\delta R_{\mathrm{cut}}=2\delta R$ when
$r_{\mathrm{bkg}}<2\delta R$, otherwise $\delta R_{\mathrm{cut}}=\delta R$.
FixedPenalty uses

$$
n_R=\max\!\left(1,
\left\lfloor 1+\frac{a\pi}{2\,\delta R_{\mathrm{cut}}\,\Delta k}\right\rfloor
\right).
$$

The residual takes the first $n_R$ available complex FFT bins, starting with
R=0, and stacks their real and imaginary parts. Thus $m$ is twice the retained
bin count, including the zero imaginary part at R=0. The actual bin spacing
remains $\delta R$. An explicit `nknots` changes the coefficient count after
this cutoff is chosen; it does not change $n_R$. The formulas trace
[`AUTOBK::calc_background`](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/background.rs) and
[`Geometry::head`](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/background/fixed.rs); the special
small-cutoff handling is a rexafs choice, not a general AUTOBK identity.
