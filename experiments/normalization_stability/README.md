# Normalization stability prototype

This experiment compares an independent Victoreen/quadratic reference
normalization against three step-aware alternatives:

1. a C1-continuous post-edge constrained to the separately fitted pre-edge;
2. a jointly fitted C1-continuous piecewise model; and
3. one shared quadratic baseline plus a constant edge step.

The independent and C1 methods use the two-term Victoreen pre-edge curve
`a(E0/E)^3 + b(E0/E)^4`. All methods fit only observations in the selected
pre-edge and post-edge windows. The edge/XANES gap remains excluded.

Run it from anywhere in the repository with:

```sh
uv run experiments/normalization_stability/compare.py
```

The script uses PEP 723 inline metadata, so `uv` creates the environment and
installs NumPy automatically. Results are written to `results/`:

- `report.md`: concise comparison tables and metric definitions;
- `summary.json`: machine-readable aggregate metrics;
- `window_runs.csv`: one row per dataset, method, and window selection;
- `nominal_curves.csv`: fitted baselines, normalized spectra, and flattened
  spectra for the default windows; and
- `interactive-normalization.html`: interactive comparison using the actual Ru
  and Cu test spectra, including an adjustable E0.

The script asserts exact first-derivative continuity for all three constrained
models.

## Scope and reproducibility

This is an isolated research prototype recovered from the September 2026 work.
The `current` label in the script means the independent Victoreen/quadratic
reference model used in this experiment. It does **not** call the Rust library
and does not represent the automatic defaults of the current `PrePostEdge()`.
The two measured spectra and fixed E0 choices are defined in `load_datasets`.
No production normalization model is changed by retaining this experiment.

The small [report](results/report.md) and [summary](results/summary.json) are
retained. Generated CSV files and HTML outputs are reproducible and ignored by
Git. The full original research archive is kept outside the source repository.
For a clean rerun without overwriting retained summaries:

```bash
uv run experiments/normalization_stability/compare.py --output-dir /tmp/rexafs-normalization
```

The pinned NumPy version in the script makes the numerical environment explicit.

A September 11, 2026 rerun with CPython 3.14.2 and NumPy 2.5.3 reproduced
all retained numerical aggregates within `rtol=1e-10, atol=1e-12`. Source-path
metadata was updated from the former crate name to `crates/rexafs`.

## Models and what continuity means

This experiment introduces candidate models; it does not claim that the
constraints below are established properties of every absorption edge.
See the [production processing theory](../../doc/processing-theory.md) for the
actual rexafs normalization model and its references.

Let $x=(E-E_0)/(1000\,\mathrm{eV})$ be dimensionless energy relative to the edge,
and define the experiment's Victoreen basis as

$$
V(E)=a_3(E_0/E)^3+a_4(E_0/E)^4.
$$

$E$ and $E_0$ are in eV; $a_3$ and $a_4$ have the units of the input absorption.
This particular two-term basis is part of the experiment. It should not be
confused with the default straight-line pre-edge fit in `PrePostEdge()`.

| Model | Pre-edge baseline | Post-edge baseline | How coefficients are fitted |
|---|---|---|---|
| Independent reference | $V(E)$ | $b_0+b_1x+b_2x^2$ | Fit the two regions separately |
| C1 anchored | $V(E)$ | $V(E)+s+cx^2$ | Fit $V$ to pre-edge data, then fit $s,c$ to post-edge data |
| C1 joint | $V(E)$ | $V(E)+s+cx^2$ | Fit all four coefficients to both selected regions together |
| Shared polynomial | $a+bx+cx^2$ | $a+bx+cx^2+s$ | Fit all four coefficients to both regions together |

All coefficients in this table have absorption units because $x$ is
dimensionless. The step $s$ is fitted, not prescribed. For the independent
reference, it is the difference between the two fitted baselines at $E_0$.

“C1” means that value and first derivative are continuous **after subtracting
the edge step** from the post-edge model. In the anchored and joint models,
$x^2$ and its first derivative vanish at $E_0$, so the two sides meet with the
same value and slope after that subtraction. This is an algebraic consequence
of the chosen model, not evidence that the assumption improves physical accuracy.
A joint fit can change the pre-edge coefficients because post-edge data enter
the same least-squares problem. The fitted observation sets exclude the edge gap.
The implementations are [`fit_current`, `fit_c1_anchored`, `fit_c1_joint` and
`fit_shared_polynomial`](compare.py).

## Meaning of the stability metrics

For each model and spectrum, the 625 window combinations give fitted steps
$s_1,\ldots,s_{625}$. The reported relative step spread is

$$
100\,\frac{\operatorname{SD}(s_1,\ldots,s_{625})}
{|\operatorname{median}(s_1,\ldots,s_{625})|}\ \%.
$$

The script uses population standard deviation across the enumerated window
choices (`ddof=0`). The 5–95% span instead uses the difference between the 95th
and 5th percentiles, divided by the same absolute median. Neither is a statistical
confidence interval from repeated independent measurements.

For a normalized curve $z_b(E_j)$ from window combination $b$, its RMS spread is

$$
\sqrt{\frac1L\sum_{j=1}^{L}
\operatorname{SD}_{b}[z_b(E_j)]^2}.
$$

$L$ is the number of energies in the script's common evaluation region. The
result is dimensionless and summarizes sensitivity to window selection.
Baseline root-mean-square error (RMSE) instead compares measured absorption
with a fitted pre/post-edge curve, then divides by the absolute fitted step.
Low window sensitivity and low baseline error are different properties;
neither alone demonstrates that the recovered EXAFS background is correct.
