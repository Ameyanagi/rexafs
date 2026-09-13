# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "numpy==2.5.3",
# ]
# ///
"""Compare edge-normalization models under pre/post window perturbations.

This is deliberately an isolated research prototype.  It uses the spectra that
already live in ``crates/rexafs/tests/testfiles`` and does not alter the
Rust normalization implementation.

The four models are:

* ``current``: independent Victoreen pre-edge and quadratic post-edge fits.
* ``c1_anchored``: fit the pre-edge Victoreen curve first, then fit an edge step and
  post-edge curvature while forcing value and first-derivative continuity at
  E0 after subtracting the step.
* ``c1_joint``: the same C1-continuous piecewise model, but fit all parameters
  to both selected regions in one least-squares solve.
* ``shared_polynomial``: one quadratic baseline across both selected regions,
  plus a constant edge step on the post-edge observations.

Only observations inside the selected pre-edge and post-edge windows enter a
fit.  No observations in the edge/XANES gap are fitted.

Energies and window offsets are in eV. Fits use unweighted linear least squares
with numpy.linalg.lstsq(..., rcond=None); the candidate constraints are properties
of this prototype, not established physical continuity laws. The adjacent
README.md defines the models and metrics and links the production theory.
Numerical solve reference:
https://numpy.org/doc/stable/reference/generated/numpy.linalg.lstsq.html
"""

from __future__ import annotations

import argparse
import csv
import json
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from itertools import product
from pathlib import Path

import numpy as np
from numpy.typing import NDArray

FloatArray = NDArray[np.float64]
SCALE_EV = 1000.0


METHOD_LABELS = {
    "current": "Independent Victoreen / quadratic",
    "c1_anchored": "C1 anchored to pre-edge",
    "c1_joint": "C1 joint fit",
    "shared_polynomial": "Shared quadratic + step",
}


@dataclass(frozen=True)
class Dataset:
    """Measured energy/absorption arrays and a fixed edge energy in eV.

    source records a repository-relative provenance path. Absorption may have
    arbitrary measurement units; normalized outputs are dimensionless. The
    frozen dataclass prevents field reassignment, not mutation of its arrays.
    """

    key: str
    label: str
    source: str
    energy: FloatArray
    mu: FloatArray
    e0: float

    @property
    def relative_energy(self) -> FloatArray:
        return self.energy - self.e0


@dataclass(frozen=True)
class Window:
    """Pre/post fit endpoints expressed as eV offsets from Dataset.e0.

    _regions maps these offsets to end-exclusive sample slices. The edge gap
    between the two regions is not fitted.
    """

    pre_start: float
    pre_end: float
    post_start: float
    post_end: float


@dataclass
class FitResult:
    """Fitted baselines, step, normalized curves, and continuity diagnostics.

    Baselines, edge_step and model coefficients have the input absorption units.
    norm and flat are dimensionless; slopes and derivative_jump are absorption
    units per eV. Baseline/curve arrays cover the full input energy grid. These
    results contain no parameter covariance or measurement-noise estimate.
    """

    edge_step: float
    pre_edge: FloatArray
    post_edge: FloatArray
    norm: FloatArray
    flat: FloatArray
    pre_slope_at_e0: float
    post_slope_at_e0: float
    coefficients: list[float]

    @property
    def derivative_jump(self) -> float:
        return self.post_slope_at_e0 - self.pre_slope_at_e0


@dataclass(frozen=True)
class Sweep:
    """Named collection of endpoint combinations for a window-sensitivity study."""

    key: str
    label: str
    windows: tuple[Window, ...]


def _load_table(path: Path) -> FloatArray:
    data = np.loadtxt(path, comments="#", dtype=np.float64)
    if data.ndim != 2 or len(data) < 10:
        raise ValueError(f"not enough tabular data in {path}")
    return data


def load_datasets(repo_root: Path) -> list[Dataset]:
    """Load retained Ru QAS and Cu foil fixtures with fixed reference E0 values.

    Ru absorption is the natural logarithm of columns I0/It; Cu absorption is
    read directly from the fixture's second column. These data are reused for
    comparing candidate models, not synthetic truth for a baseline. No library
    normalization or automatic edge finder is called here. File/parse errors
    propagate from NumPy.
    """
    testfiles = repo_root / "crates/rexafs/tests/testfiles"

    ru_path = testfiles / "Ru_QAS.dat"
    ru_data = _load_table(ru_path)
    ru = Dataset(
        key="ru_qas",
        label="Ru K edge (QAS)",
        source=str(ru_path.relative_to(repo_root)),
        energy=ru_data[:, 0],
        mu=np.log(ru_data[:, 1] / ru_data[:, 2]),
        # This is the E0 found and used by the existing Rust regression test.
        e0=22118.8,
    )

    cu_path = testfiles / "xraylarch_d867/xafsdata/cu_150k.xmu"
    cu_data = _load_table(cu_path)
    cu = Dataset(
        key="cu_150k",
        label="Cu K edge (150 K foil)",
        source=str(cu_path.relative_to(repo_root)),
        energy=cu_data[:, 0],
        mu=cu_data[:, 1],
        # The file header gives E0 = 8980.0 eV.
        e0=8980.0,
    )

    return [ru, cu]


def _nearest_index(values: FloatArray, target: float) -> int:
    insertion = int(np.searchsorted(values, target, side="left"))
    if insertion <= 0:
        return 0
    if insertion >= len(values):
        return len(values) - 1
    before = insertion - 1
    if abs(values[before] - target) <= abs(values[insertion] - target):
        return before
    return insertion


def _rust_style_slice(energy: FloatArray, start: float, end: float) -> slice:
    """Map eV boundaries using the prototype's retained end-exclusive convention.

    Start at the last sample no greater than start, or index zero. Stop before
    the nearest sample to end; _nearest_index breaks a tie toward the earlier
    sample. This can include a point below the requested lower boundary. Raise
    ValueError if the resulting slice is empty.
    """

    first = max(0, int(np.searchsorted(energy, start, side="right")) - 1)
    last = _nearest_index(energy, end)
    if last <= first:
        raise ValueError(f"empty fit window [{start}, {end}]")
    return slice(first, last)


def _regions(dataset: Dataset, window: Window) -> tuple[slice, slice, FloatArray]:
    pre = _rust_style_slice(
        dataset.energy,
        dataset.e0 + window.pre_start,
        dataset.e0 + window.pre_end,
    )
    post = _rust_style_slice(
        dataset.energy,
        dataset.e0 + window.post_start,
        dataset.e0 + window.post_end,
    )
    scaled_energy = dataset.relative_energy / SCALE_EV
    if len(scaled_energy[pre]) < 3:
        raise ValueError("the pre-edge window must contain at least 3 points")
    if len(scaled_energy[post]) < 4:
        raise ValueError("the post-edge window must contain at least 4 points")
    return pre, post, scaled_energy


def _victoreen_basis(energy: FloatArray, e0: float) -> FloatArray:
    """Return a well-scaled basis for a/E^3 + b/E^4.

    Using (E0/E)^3 and (E0/E)^4 keeps the fitted coefficients near the scale
    of mu without changing the two-term curve chosen for this experiment.
    """

    ratio = e0 / energy
    return np.column_stack((ratio**3, ratio**4))


def _victoreen_curve(
    energy: FloatArray, e0: float, coefficients: FloatArray
) -> FloatArray:
    return _victoreen_basis(energy, e0) @ coefficients


def _victoreen_slope_at_e0(e0: float, coefficients: FloatArray) -> float:
    return float(-(3.0 * coefficients[0] + 4.0 * coefficients[1]) / e0)


def _finish_fit(
    dataset: Dataset,
    edge_step: float,
    pre_edge: FloatArray,
    post_edge: FloatArray,
    pre_slope_at_e0: float,
    post_slope_at_e0: float,
    coefficients: Iterable[float],
) -> FitResult:
    """Build dimensionless normalized/flattened outputs on the measured grid.

    Normalize as (mu - pre_edge)/edge_step. From the sample nearest E0 onward,
    flat instead uses (mu - post_edge)/edge_step + 1; earlier flat samples retain
    norm. Reject nonfinite steps and steps <= 1e-12 absorption units. Baseline
    arrays are retained by reference; norm and flat are newly allocated arrays.
    """
    if not np.isfinite(edge_step) or edge_step <= 1.0e-12:
        raise ValueError(f"invalid edge step: {edge_step}")

    norm = (dataset.mu - pre_edge) / edge_step
    flat = norm.copy()
    ie0 = _nearest_index(dataset.energy, dataset.e0)
    flat[ie0:] = (dataset.mu[ie0:] - post_edge[ie0:]) / edge_step + 1.0

    return FitResult(
        edge_step=float(edge_step),
        pre_edge=pre_edge,
        post_edge=post_edge,
        norm=norm,
        flat=flat,
        pre_slope_at_e0=float(pre_slope_at_e0),
        post_slope_at_e0=float(post_slope_at_e0),
        coefficients=[float(value) for value in coefficients],
    )


def fit_current(dataset: Dataset, window: Window) -> FitResult:
    """Fit independent two-term pre-edge and quadratic post-edge baselines.

    ``current`` is a historical model label, not the current Rust normalization
    implementation. Each selected region receives a separate unweighted solve;
    the fitted edge step is the post-minus-pre baseline difference at E0.
    """

    pre, post, x = _regions(dataset, window)
    x_post = x[post]

    pre_coef = np.linalg.lstsq(
        _victoreen_basis(dataset.energy[pre], dataset.e0), dataset.mu[pre], rcond=None
    )[0]
    post_coef = np.linalg.lstsq(
        np.column_stack((np.ones(len(x_post)), x_post, x_post**2)),
        dataset.mu[post],
        rcond=None,
    )[0]

    pre_edge = _victoreen_curve(dataset.energy, dataset.e0, pre_coef)
    post_edge = post_coef[0] + post_coef[1] * x + post_coef[2] * x**2
    edge_step = post_coef[0] - float(np.sum(pre_coef))
    return _finish_fit(
        dataset,
        edge_step,
        pre_edge,
        post_edge,
        _victoreen_slope_at_e0(dataset.e0, pre_coef),
        post_coef[1] / SCALE_EV,
        np.concatenate((pre_coef, post_coef)),
    )


def fit_c1_anchored(dataset: Dataset, window: Window) -> FitResult:
    """Preserve the Victoreen pre-edge, then optimize step and post curvature.

    With E and E0 in eV and dimensionless x = (E-E0)/(1000 eV), the model is

        pre:  V(E) = a3*(E0/E)^3 + a4*(E0/E)^4
        post: V(E) + step + a2*x^2

    so subtracting ``step`` from the post-edge model makes its value and first
    derivative match the pre-edge curve at E0. All fitted coefficients have
    absorption units. Fit a3/a4 only to the pre-edge observations, then hold them
    fixed while fitting step/a2 to the post-edge observations.
    """

    pre, post, x = _regions(dataset, window)
    x_post = x[post]

    pre_coef = np.linalg.lstsq(
        _victoreen_basis(dataset.energy[pre], dataset.e0), dataset.mu[pre], rcond=None
    )[0]
    pre_edge = _victoreen_curve(dataset.energy, dataset.e0, pre_coef)
    post_minus_curve = dataset.mu[post] - pre_edge[post]
    step, curvature = np.linalg.lstsq(
        np.column_stack((np.ones(len(x_post)), x_post**2)),
        post_minus_curve,
        rcond=None,
    )[0]

    post_edge = pre_edge + step + curvature * x**2
    slope = _victoreen_slope_at_e0(dataset.e0, pre_coef)
    return _finish_fit(
        dataset,
        step,
        pre_edge,
        post_edge,
        slope,
        slope,
        (*pre_coef, step, curvature),
    )


def fit_c1_joint(dataset: Dataset, window: Window) -> FitResult:
    """Fit all four coefficients of the C1 model to both regions together.

    The pre-edge is V(E); the post-edge is V(E) + step + curvature*x**2, with
    dimensionless x defined in fit_c1_anchored. After subtracting step, values
    and slopes meet at E0. Each observation has equal weight, so the region with
    more samples contributes more residual terms. No noise weights are fitted.
    """

    pre, post, x = _regions(dataset, window)
    x_post = x[post]

    victoreen_pre = _victoreen_basis(dataset.energy[pre], dataset.e0)
    victoreen_post = _victoreen_basis(dataset.energy[post], dataset.e0)
    design_pre = np.column_stack(
        (
            victoreen_pre,
            np.zeros(len(victoreen_pre)),
            np.zeros(len(victoreen_pre)),
        )
    )
    design_post = np.column_stack((victoreen_post, np.ones(len(x_post)), x_post**2))
    coefficients = np.linalg.lstsq(
        np.vstack((design_pre, design_post)),
        np.concatenate((dataset.mu[pre], dataset.mu[post])),
        rcond=None,
    )[0]
    victoreen_coef = coefficients[:2]
    step, curvature = coefficients[2:]

    pre_edge = _victoreen_curve(dataset.energy, dataset.e0, victoreen_coef)
    post_edge = pre_edge + step + curvature * x**2
    slope = _victoreen_slope_at_e0(dataset.e0, victoreen_coef)
    return _finish_fit(
        dataset,
        step,
        pre_edge,
        post_edge,
        slope,
        slope,
        coefficients,
    )


def fit_shared_polynomial(dataset: Dataset, window: Window) -> FitResult:
    """Fit one quadratic baseline and a constant post-edge offset jointly.

    The shared polynomial is intercept + slope*x + curvature*x**2 with
    dimensionless x = (E-E0)/(1000 eV). A fourth coefficient adds step only to
    post-edge observations. All observations have equal weight and all four
    coefficients have absorption units. Sharing curvature is a prototype
    assumption; a good fit does not establish a physically correct background.
    """

    pre, post, x = _regions(dataset, window)
    x_pre = x[pre]
    x_post = x[post]

    design_pre = np.column_stack(
        (np.ones(len(x_pre)), x_pre, x_pre**2, np.zeros(len(x_pre)))
    )
    design_post = np.column_stack(
        (np.ones(len(x_post)), x_post, x_post**2, np.ones(len(x_post)))
    )
    coefficients = np.linalg.lstsq(
        np.vstack((design_pre, design_post)),
        np.concatenate((dataset.mu[pre], dataset.mu[post])),
        rcond=None,
    )[0]
    intercept, slope_scaled, curvature, step = coefficients

    pre_edge = intercept + slope_scaled * x + curvature * x**2
    post_edge = pre_edge + step
    slope = slope_scaled / SCALE_EV
    return _finish_fit(
        dataset,
        step,
        pre_edge,
        post_edge,
        slope,
        slope,
        coefficients,
    )


FIT_METHODS: dict[str, Callable[[Dataset, Window], FitResult]] = {
    "current": fit_current,
    "c1_anchored": fit_c1_anchored,
    "c1_joint": fit_c1_joint,
    "shared_polynomial": fit_shared_polynomial,
}


def make_sweeps(dataset: Dataset) -> list[Sweep]:
    """Return two factorial sweeps of pre/post endpoints relative to E0 in eV.

    Five choices per endpoint give 625 windows per sweep for the included data.
    The first sweep drops post-end choices beyond the dataset's available span;
    the second chooses five ends from max(300 eV, 40% of that span) to its end.
    This deterministic sensitivity grid is not a random uncertainty sample.
    """
    pre_starts = (-200.0, -180.0, -160.0, -140.0, -120.0)
    pre_ends = (-90.0, -75.0, -65.0, -50.0, -35.0)
    post_starts = (25.0, 50.0, 75.0, 100.0, 150.0)

    relative_max = float(dataset.relative_energy[-1])
    near_post_ends = tuple(
        value for value in (300.0, 500.0, 700.0, 850.0, 940.0) if value <= relative_max
    )
    available_start = max(300.0, 0.4 * relative_max)
    available_post_ends = tuple(
        float(value) for value in np.linspace(available_start, relative_max, 5)
    )

    def build(ends: tuple[float, ...]) -> tuple[Window, ...]:
        return tuple(
            Window(*values)
            for values in product(pre_starts, pre_ends, post_starts, ends)
        )

    return [
        Sweep(
            key="near_post_edge",
            label="Post-edge end varied from 300 to 940 eV",
            windows=build(near_post_ends),
        ),
        Sweep(
            key="available_span",
            label="Post-edge end varied over 40–100% of available span",
            windows=build(available_post_ends),
        ),
    ]


def nominal_window(dataset: Dataset) -> Window:
    """Use the experiment's reference windows, independent of library defaults.

    The pre-edge range is [-200, -65] eV relative to E0; the post-edge range is
    [25 eV, last measured energy - E0]. Sample boundaries use _regions.
    """
    return Window(-200.0, -65.0, 25.0, float(dataset.relative_energy[-1]))


def _evaluation_slices(dataset: Dataset, sweep: Sweep) -> tuple[slice, slice]:
    post_end = min(
        max(window.post_end for window in sweep.windows), dataset.relative_energy[-1]
    )
    return (
        _rust_style_slice(dataset.energy, dataset.e0 - 200.0, dataset.e0 - 35.0),
        _rust_style_slice(dataset.energy, dataset.e0 + 25.0, dataset.e0 + post_end),
    )


def _rms(values: FloatArray) -> float:
    return float(np.sqrt(np.mean(values**2)))


def analyze_sweep(
    dataset: Dataset,
    sweep: Sweep,
) -> tuple[dict[str, dict[str, float | int]], list[dict[str, float | str]]]:
    """Summarize window sensitivity and return successful per-window records.

    SD uses population normalization (ddof=0) across successful fits. Failed
    windows are counted and omitted; all failures for a model raise RuntimeError.
    Curve spread is RMS pointwise SD, over 0 to at most 800 eV above E0 for norm
    and 50 to at most 800 eV for flat. Neither spread is a confidence interval.
    Baseline RMSE is evaluated on common regions, divided by each fit's positive
    step, and summarized by its median. README.md defines the full equations.
    """
    pre_eval, post_eval = _evaluation_slices(dataset, sweep)
    upper_curve_energy = min(800.0, max(window.post_end for window in sweep.windows))
    norm_eval = (dataset.relative_energy >= 0.0) & (
        dataset.relative_energy <= upper_curve_energy
    )
    flat_eval = (dataset.relative_energy >= 50.0) & (
        dataset.relative_energy <= upper_curve_energy
    )

    summary: dict[str, dict[str, float | int]] = {}
    rows: list[dict[str, float | str]] = []

    for method_key, fit_method in FIT_METHODS.items():
        fits: list[FitResult] = []
        successful_windows: list[Window] = []
        failures = 0
        for window in sweep.windows:
            try:
                fit = fit_method(dataset, window)
            except (ValueError, np.linalg.LinAlgError):
                failures += 1
                continue
            fits.append(fit)
            successful_windows.append(window)

        if not fits:
            raise RuntimeError(
                f"all {method_key} fits failed for {dataset.key}/{sweep.key}"
            )

        steps = np.asarray([fit.edge_step for fit in fits])
        norms = np.asarray([fit.norm for fit in fits])
        flats = np.asarray([fit.flat for fit in fits])
        median_step = float(np.median(steps))
        norm_spread = _rms(np.std(norms[:, norm_eval], axis=0))
        flat_spread = _rms(np.std(flats[:, flat_eval], axis=0))

        pre_residuals: list[float] = []
        post_residuals: list[float] = []
        for fit, window in zip(fits, successful_windows):
            pre_rmse = _rms(dataset.mu[pre_eval] - fit.pre_edge[pre_eval]) / abs(
                fit.edge_step
            )
            post_rmse = _rms(dataset.mu[post_eval] - fit.post_edge[post_eval]) / abs(
                fit.edge_step
            )
            pre_residuals.append(pre_rmse)
            post_residuals.append(post_rmse)
            rows.append(
                {
                    "dataset": dataset.key,
                    "sweep": sweep.key,
                    "method": method_key,
                    "pre_start_ev": window.pre_start,
                    "pre_end_ev": window.pre_end,
                    "post_start_ev": window.post_start,
                    "post_end_ev": window.post_end,
                    "edge_step": fit.edge_step,
                    "pre_rmse_over_step": pre_rmse,
                    "post_rmse_over_step": post_rmse,
                    "derivative_jump_mu_per_ev": fit.derivative_jump,
                }
            )

        summary[method_key] = {
            "runs": len(fits),
            "failures": failures,
            "edge_step_median": median_step,
            "edge_step_std_percent": 100.0 * float(np.std(steps)) / abs(median_step),
            "edge_step_p90_span_percent": 100.0
            * float(np.percentile(steps, 95.0) - np.percentile(steps, 5.0))
            / abs(median_step),
            "norm_rms_spread": norm_spread,
            "flat_rms_spread": flat_spread,
            "pre_rmse_over_step_median": float(np.median(pre_residuals)),
            "post_rmse_over_step_median": float(np.median(post_residuals)),
        }

    return summary, rows


def nominal_results(dataset: Dataset) -> dict[str, FitResult]:
    window = nominal_window(dataset)
    return {key: method(dataset, window) for key, method in FIT_METHODS.items()}


def self_check(datasets: list[Dataset]) -> None:
    """Require derivative jumps <= 1e-14 absorption units/eV at nominal windows.

    This checks the three constrained model implementations only; it does not
    establish physical continuity or audit every perturbed window fit.
    """
    # The constrained models must be C1 to floating-point precision.
    for dataset in datasets:
        window = nominal_window(dataset)
        for method in (fit_c1_anchored, fit_c1_joint, fit_shared_polynomial):
            fit = method(dataset, window)
            if abs(fit.derivative_jump) > 1.0e-14:
                raise AssertionError(f"{method.__name__} is not derivative-continuous")


def write_window_runs(path: Path, rows: list[dict[str, float | str]]) -> None:
    fieldnames = [
        "dataset",
        "sweep",
        "method",
        "pre_start_ev",
        "pre_end_ev",
        "post_start_ev",
        "post_end_ev",
        "edge_step",
        "pre_rmse_over_step",
        "post_rmse_over_step",
        "derivative_jump_mu_per_ev",
    ]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def write_nominal_curves(
    path: Path,
    datasets: list[Dataset],
    fits_by_dataset: dict[str, dict[str, FitResult]],
) -> None:
    fieldnames = ["dataset", "energy_ev", "relative_energy_ev", "mu"]
    for method_key in FIT_METHODS:
        fieldnames.extend(
            (
                f"{method_key}_pre_edge",
                f"{method_key}_post_edge",
                f"{method_key}_norm",
                f"{method_key}_flat",
            )
        )

    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for dataset in datasets:
            fits = fits_by_dataset[dataset.key]
            for index in range(len(dataset.energy)):
                row: dict[str, float | str] = {
                    "dataset": dataset.key,
                    "energy_ev": float(dataset.energy[index]),
                    "relative_energy_ev": float(dataset.relative_energy[index]),
                    "mu": float(dataset.mu[index]),
                }
                for method_key, fit in fits.items():
                    row[f"{method_key}_pre_edge"] = float(fit.pre_edge[index])
                    row[f"{method_key}_post_edge"] = float(fit.post_edge[index])
                    row[f"{method_key}_norm"] = float(fit.norm[index])
                    row[f"{method_key}_flat"] = float(fit.flat[index])
                writer.writerow(row)


def _fmt(value: float, digits: int = 3) -> str:
    return f"{value:.{digits}f}"


def write_report(path: Path, payload: dict[str, object]) -> None:
    datasets = payload["datasets"]
    lines = [
        "# Normalization stability experiment",
        "",
        "The fit uses only the selected pre-edge and post-edge observations; the edge/XANES gap is excluded.",
        "Each sweep is a 5 × 5 × 5 × 5 full-factorial variation of the four window endpoints (625 fits per method).",
        "",
    ]

    for dataset_index, (dataset_key, dataset_data) in enumerate(datasets.items()):
        if dataset_index:
            lines.append("")
        lines.extend((f"## {dataset_data['label']}", ""))
        nominal = dataset_data["nominal"]
        lines.extend(
            (
                "Nominal/default-window edge steps:",
                "",
                "| Method | Edge step | Derivative jump at E0 (μ/eV) |",
                "|---|---:|---:|",
            )
        )
        for method_key in FIT_METHODS:
            item = nominal[method_key]
            lines.append(
                f"| {METHOD_LABELS[method_key]} | {_fmt(item['edge_step'], 6)} | {item['derivative_jump_mu_per_ev']:.3e} |"
            )

        for sweep_data in dataset_data["sweeps"].values():
            lines.extend(
                (
                    "",
                    f"### {sweep_data['label']}",
                    "",
                    "| Method | Step SD / median | Step 5–95% span | Norm RMS spread | Flat RMS spread | Pre RMSE / step | Post RMSE / step |",
                    "|---|---:|---:|---:|---:|---:|---:|",
                )
            )
            for method_key in FIT_METHODS:
                item = sweep_data["methods"][method_key]
                lines.append(
                    "| {} | {}% | {}% | {} | {} | {} | {} |".format(
                        METHOD_LABELS[method_key],
                        _fmt(item["edge_step_std_percent"]),
                        _fmt(item["edge_step_p90_span_percent"]),
                        _fmt(item["norm_rms_spread"], 4),
                        _fmt(item["flat_rms_spread"], 4),
                        _fmt(item["pre_rmse_over_step_median"], 4),
                        _fmt(item["post_rmse_over_step_median"], 4),
                    )
                )

    lines.extend(
        (
            "",
            "## Metric definitions",
            "",
            "- **Step SD / median**: standard deviation of fitted edge steps divided by the median step.",
            "- **Step 5–95% span**: central 90% edge-step range divided by the median step.",
            "- **Norm/flat RMS spread**: RMS over energy of the pointwise standard deviation across all window selections.",
            "- **Pre/post RMSE / step**: median common-window baseline residual divided by that run's edge step.",
            "",
        )
    )
    path.write_text("\n".join(lines), encoding="utf-8")


def write_interactive_example(path: Path, datasets: list[Dataset]) -> None:
    """Write a standalone browser comparison using the local HTML template.

    Embed measured points with relative energies rounded to four decimal places
    in eV and absorption rounded to eight decimal places. This compact display
    payload does not replace the full-precision arrays used for report metrics.
    The template must contain exactly one spectra marker or RuntimeError is
    raised. An existing output file is overwritten.
    """
    template_path = Path(__file__).resolve().parent / "interactive-template.html"
    template = template_path.read_text(encoding="utf-8")
    marker = "/*__SPECTRA__*/"
    if template.count(marker) != 1:
        raise RuntimeError(f"expected one {marker} marker in {template_path}")

    spectra = {
        dataset.key: {
            "label": dataset.label,
            "e0": dataset.e0,
            "points": [
                [round(float(relative), 4), round(float(mu), 8)]
                for relative, mu in zip(dataset.relative_energy, dataset.mu)
            ],
        }
        for dataset in datasets
    }
    rendered = template.replace(
        marker,
        json.dumps(spectra, separators=(",", ":"), sort_keys=True),
    )
    path.write_text(rendered, encoding="utf-8")


def run(repo_root: Path, output_dir: Path) -> dict[str, object]:
    """Run both measured datasets and write JSON, CSV, Markdown and HTML outputs.

    Return the same summary payload written to summary.json. The directory is
    created as needed and existing named outputs are overwritten; use a separate
    output directory to preserve archived evidence. Model/input/file errors
    propagate, with per-window fit failures handled by analyze_sweep.
    """
    datasets = load_datasets(repo_root)
    self_check(datasets)

    payload: dict[str, object] = {
        "method_labels": METHOD_LABELS,
        "datasets": {},
    }
    all_rows: list[dict[str, float | str]] = []
    fits_by_dataset: dict[str, dict[str, FitResult]] = {}

    for dataset in datasets:
        nominal = nominal_results(dataset)
        fits_by_dataset[dataset.key] = nominal
        dataset_payload: dict[str, object] = {
            "label": dataset.label,
            "source": dataset.source,
            "e0_ev": dataset.e0,
            "nominal_window": vars(nominal_window(dataset)),
            "nominal": {
                method_key: {
                    "edge_step": fit.edge_step,
                    "pre_slope_mu_per_ev": fit.pre_slope_at_e0,
                    "post_slope_mu_per_ev": fit.post_slope_at_e0,
                    "derivative_jump_mu_per_ev": fit.derivative_jump,
                }
                for method_key, fit in nominal.items()
            },
            "sweeps": {},
        }

        for sweep in make_sweeps(dataset):
            method_summary, rows = analyze_sweep(dataset, sweep)
            all_rows.extend(rows)
            dataset_payload["sweeps"][sweep.key] = {
                "label": sweep.label,
                "window_count": len(sweep.windows),
                "methods": method_summary,
            }

        payload["datasets"][dataset.key] = dataset_payload

    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "summary.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    write_window_runs(output_dir / "window_runs.csv", all_rows)
    write_nominal_curves(output_dir / "nominal_curves.csv", datasets, fits_by_dataset)
    write_report(output_dir / "report.md", payload)
    write_interactive_example(output_dir / "interactive-normalization.html", datasets)
    return payload


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).resolve().parent / "results",
        help="directory for JSON, CSV, and Markdown results",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    payload = run(repo_root, args.output_dir.resolve())
    print(
        f"wrote comparison for {len(payload['datasets'])} datasets to {args.output_dir.resolve()}"
    )


if __name__ == "__main__":
    main()
