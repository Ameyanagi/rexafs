---
marp: true
theme: default
header: 'Likelihood and uncertainty'
paginate: true
---

# Likelihood and uncertainty

Revised September 11, 2026. These notes replace the earlier unverified claim
about a particular Larch/IFEFFIT call chain. The derivation is general; current
rexafs behavior is described in [fitting statistics](../doc/fitting-statistics.md).

---

# Gaussian measurement model

Assume independent observations $y_i$ with model means $g_i(\boldsymbol\theta)$
and known, parameter-independent standard deviations $s_i>0$. Their likelihood is

$$
L(\boldsymbol\theta)=\prod_{i=1}^{M}
\frac{1}{\sqrt{2\pi}s_i}
\exp\!\left[-\frac{(y_i-g_i(\boldsymbol\theta))^2}{2s_i^2}\right].
$$

$M$ is the observation count and $\boldsymbol\theta$ contains the $p$ unknown
parameters. Independence permits the product. Correlated measurements require
a joint noise model rather than this product of scalar densities.

---

# Why weighted least squares appears

Taking the negative logarithm gives

$$
-\log L=\frac12\sum_{i=1}^{M}
\left(\frac{y_i-g_i(\boldsymbol\theta)}{s_i}\right)^2
+\sum_{i=1}^{M}\log(\sqrt{2\pi}s_i).
$$

Because the second term does not depend on the parameters under our assumptions,
maximizing likelihood is equivalent to minimizing weighted squared residuals.
If the noise variance depends on the parameters, that second term cannot simply
be discarded. This is the algebraic link to the
[least-squares objective](uncertainty.md).

---

# Stationarity and curvature

Write $\ell=\log L$. At an unconstrained, differentiable interior optimum,
$\nabla\ell=0$. A negative-definite Hessian of $\ell$ is sufficient for a
strict local maximum. It is not a necessary condition for all maxima: a maximum
can have zero curvature in a direction. Solving the score equation alone does
not establish a maximum or uniqueness.

---

# Local uncertainty

Let $\widehat{\boldsymbol\theta}$ be the fitted point and
$H=\nabla^2[-\ell](\widehat{\boldsymbol\theta})$. A local quadratic expansion is

$$
-\ell(\boldsymbol\theta)\approx-\ell(\widehat{\boldsymbol\theta})+
\frac12(\boldsymbol\theta-\widehat{\boldsymbol\theta})^T
H(\boldsymbol\theta-\widehat{\boldsymbol\theta}).
$$

When $H$ is positive definite and the approximation is appropriate, $H^{-1}$
provides a local covariance approximation under the measurement model. The
sampling distribution of an estimator and a posterior distribution with a
chosen prior are different objects; this expansion does not by itself supply
an exact Bayesian posterior or a guaranteed confidence interval.

---

# Relation to software outputs

[SciPy's `leastsq` documentation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.leastsq.html)
describes a Jacobian-based inverse-curvature estimate and the need to scale it
by residual variance when appropriate. A singular estimate can be unavailable.
This is a software convention to inspect, not a reason to omit the noise model.

rexafs uses raw residuals and an EXAFS independent-information estimate for
its covariance scale; read the [actual formulas](../doc/fitting-statistics.md).
The earlier notes' references to an unspecified Larch or IFEFFIT implementation
are not retained as verified facts.
