---
marp: true
theme: default
header: 'Least-squares uncertainty'
paginate: true
---

# Least squares and local uncertainty

Revised September 11, 2026. These teaching notes correct the earlier draft's
residual notation, Hessian scaling and QR permutation. They explain the
mathematics; [rexafs fitting statistics](../doc/fitting-statistics.md) documents
the production implementation's EXAFS-specific scaling.

---

# Define the problem

Let $y_i$ be observation $i$, $g_i(\boldsymbol\theta)$ its model prediction,
and $s_i>0$ its measurement standard deviation. Define

$$
r_i(\boldsymbol\theta)=\frac{y_i-g_i(\boldsymbol\theta)}{s_i},
\qquad F(\boldsymbol\theta)=\frac12\sum_{i=1}^{M}r_i(\boldsymbol\theta)^2.
$$

The $p$ fitted parameters form $\boldsymbol\theta\in\mathbb R^p$.
The $M$ residuals are dimensionless because $s_i$ has the units of $y_i$.
The factor one half simplifies derivatives without changing the minimum.
Unweighted fitting instead uses raw discrepancies and must estimate their scale.

---

# Linearize the residuals

For a small parameter step $\mathbf h$, the first-order approximation is

$$
\mathbf r(\boldsymbol\theta+\mathbf h)
\approx\mathbf r(\boldsymbol\theta)+J\mathbf h,
\qquad J_{ij}=\frac{\partial r_i}{\partial\theta_j}.
$$

The Jacobian $J$ is an $M\times p$ matrix. The step $h_j$ has the units of
parameter $j$, so $J\mathbf h$ has the units of the residuals. Curvature can make
this approximation inaccurate for large steps.

---

# Gauss–Newton and Levenberg–Marquardt steps

Minimizing the squared norm of the linearized residual gives

$$
J^TJ\mathbf h=-J^T\mathbf r.
$$

A damped Levenberg–Marquardt step instead satisfies

$$
(J^TJ+\alpha D^TD)\mathbf h=-J^T\mathbf r.
$$

Here $D$ is a diagonal parameter-scaling matrix and $\alpha\ge0$ controls
step damping. Scaling makes steps in different parameter units comparable.
Trust-region implementations adapt the allowed step using the agreement
between predicted and actual improvement. The
[MINPACK LMDIF reference](https://www.math.utah.edu/software/minpack/minpack/lmdif.html)
describes its modified Levenberg–Marquardt method and stopping rules.

---

# A stationary point is not a proof of a minimum

For an unconstrained interior stationary point,

$$
\nabla F=J^T\mathbf r=0.
$$

A minimum, maximum or saddle point can satisfy this equation. A positive-definite
Hessian is sufficient for a strict local minimum, but not necessary for every
minimum. Bounds require appropriate constrained optimality conditions.
Neither stationarity nor a solver success flag establishes a global optimum.

---

# Exact Hessian versus approximation

Differentiating the explicitly defined half-squared objective gives

$$
\nabla^2F=J^TJ+\sum_{i=1}^{M}r_i\nabla^2r_i.
$$

Gauss–Newton neglects the second term. That is exact for residuals linear in
parameters and can be useful when the omitted term is small. If the objective
is defined without the factor one half, **both terms acquire a factor of two**.
Writing $J^TJ$ as the exact Hessian of an arbitrary nonlinear squared residual
would be incorrect.

---

# Numerical Jacobians

A forward-difference estimate for column $j$ is

$$
J_{:j}\approx
\frac{\mathbf r(\boldsymbol\theta+\varepsilon_j\mathbf e_j)
-\mathbf r(\boldsymbol\theta)}{\varepsilon_j}.
$$

The vector $\mathbf e_j$ selects coordinate $j$; $\varepsilon_j$ is a finite
step in that parameter's units. Too large a step introduces truncation error;
too small a step magnifies floating-point cancellation. This is an estimate of
one **column**, not of the entire matrix at once. LMDIF estimates derivatives;
an analytic Jacobian can be supplied to the related LMDER interface.
See [SciPy's MINPACK wrapper](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.leastsq.html).

---

# Covariance in the locally linear model

With independent Gaussian errors of known standard deviations $s_i$, a
full-column-rank Jacobian gives the approximation

$$
C\approx(J^TJ)^{-1}.
$$

For unweighted, independent errors with a common unknown variance, estimate
$s^2=\sum_i r_i^2/(M-p)$ from raw residuals and use
$C\approx s^2(J^TJ)^{-1}$, provided $M>p$. These assumptions do not hold merely
because data have been interpolated onto a dense grid. The production EXAFS
code uses the different information-count scaling documented in the
[fitting guide](../doc/fitting-statistics.md).

---

# QR factorization and the permutation

For the column-pivoted factorization $JP=QR$, $P$ permutes parameter columns,
$Q^TQ=I$, and $R$ is upper triangular with nonzero diagonal for full rank.
It follows algebraically that

$$
J^TJ=P R^TR P^T,
\qquad (J^TJ)^{-1}=P(R^TR)^{-1}P^T.
$$

The transpose on the final permutation is essential: a permutation matrix
need not be symmetric. Numerical routines normally use triangular solves
rather than explicitly invert all matrices. This is a derivation of the QR
identity; rexafs currently forms and inverts $J^TJ$ in its covariance routine.

---

# Interpretation

A parameter's local standard error is $\sqrt{C_{jj}}$. Off-diagonal terms
matter: varying one parameter while fixing all others does not measure the
same uncertainty as allowing correlated parameters to vary together.

Rank deficiency, weak sensitivity, active bounds, nonlinearity and systematic
errors limit the covariance approximation. A zero or unavailable reported error
is not evidence of exact knowledge. See the
[likelihood notes](uncertainty2.md) for the statistical assumptions.
