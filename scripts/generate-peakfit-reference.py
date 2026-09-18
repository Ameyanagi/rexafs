#!/usr/bin/env -S uv run --script
# /// script
# requires-python = "==3.12.*"
# dependencies = ["numpy==2.3.2", "scipy==1.16.1", "lmfit==1.3.4"]
# ///
"""Generate synthetic XANES fit references; no experimental data is read.

Run with `uv run scripts/generate-peakfit-reference.py`. Production rexafs does
not require Python or lmfit. The profile reference is lmfit 1.3.4's lineshapes;
this script converts rexafs FWHM to its sigma/gamma convention explicitly.
Native-grid signal errors are independent synthetic Gaussian errors. They do
not represent noise after experimentally correlated normalization.
"""
import json
import platform
from pathlib import Path

import lmfit
from lmfit import lineshapes as profiles
import numpy as np
import scipy


def evaluate(p, x, shape):
    center, area, width = [p[f"p_{key}"].value for key in ("center", "area", "width")]
    args = dict(amplitude=area, center=center)
    if shape == "Gaussian":
        peak = profiles.gaussian(x, sigma=width / np.sqrt(8 * np.log(2)), **args)
    elif shape == "Lorentzian":
        peak = profiles.lorentzian(x, sigma=width / 2, **args)
    elif shape == "PseudoVoigt":
        peak = profiles.pvoigt(x, sigma=width / 2, fraction=p["p_fraction"].value, **args)
    else:
        peak = profiles.voigt(x, sigma=width / np.sqrt(8 * np.log(2)), gamma=p["p_lorentz_width"].value / 2, **args)
    baseline = p["baseline_offset"].value + p["baseline_slope"].value * x
    return peak + baseline


def make_case(shape):
    x = np.linspace(-15, 35, 251)
    x[1:-1] += 0.025 * np.sin(np.arange(1, len(x) - 1))
    sigma = 0.002 + 0.00006 * (x + 15)
    truth = lmfit.Parameters()
    truth.add("p_center", 3.2, min=-15, max=35)
    truth.add("p_area", 5.5, min=0)
    truth.add("p_width", 4.2, min=1e-8)
    truth.add("baseline_offset", 0.22)
    truth.add("baseline_slope", 0.0018)
    if shape == "PseudoVoigt":
        truth.add("p_fraction", 0.35, min=0, max=1)
    if shape == "Voigt":
        truth.add("p_lorentz_width", 1.6, min=0)
    rng = np.random.Generator(np.random.PCG64(20260917))
    signal = evaluate(truth, x, shape) + sigma * rng.standard_normal(len(x))
    initial = truth.copy()
    for name, value in {"p_center": 2., "p_area": 4.5, "p_width": 5., "baseline_offset": 0.1, "baseline_slope": 0.}.items():
        initial[name].value = value
    mask = ~((x >= 9) & (x <= 10))
    objective = lambda p: (signal[mask] - evaluate(p, x[mask], shape)) / sigma[mask]
    result = lmfit.minimize(objective, initial, method="least_squares", scale_covar=False,
                            max_nfev=2000, ftol=1e-12, xtol=1e-12, gtol=1e-12)
    assert result.success and result.covar is not None
    return dict(shape=shape, energy=(x + 9000).tolist(), signal=signal.tolist(), sigma=sigma.tolist(),
                initial={k: p.value for k, p in initial.items()}, truth={k: p.value for k, p in truth.items()},
                source_indices=np.flatnonzero(mask).tolist(), model=evaluate(result.params, x[mask], shape).tolist(),
                objective=float(np.dot(result.residual, result.residual)),
                parameters={k: dict(value=p.value, stderr=p.stderr) for k, p in result.params.items()},
                covariance_names=result.var_names, covariance=result.covar.tolist(), nfev=result.nfev)


if __name__ == "__main__":
    output = Path(__file__).resolve().parents[1] / "crates/rexafs/tests/fixtures/analysis/xanes-peaks/lmfit-reference.json"
    artifact = dict(provenance=dict(generator="scripts/generate-peakfit-reference.py", kind="synthetic",
                                   python=platform.python_version(), numpy=np.__version__, scipy=scipy.__version__,
                                   lmfit=lmfit.__version__, seed=20260917,
                                   source="https://lmfit.github.io/lmfit-py/builtin_models.html",
                                   signal="arbitrary absorption units", energy_unit="eV", origin_ev=9000,
                                   range=[-15, 35], exclude=[[9, 10]], independent_errors=True),
                    tolerances=dict(parameter_absolute=1e-4, curve_absolute=2e-7, objective_absolute=1e-6,
                                    standard_error_relative=3e-4),
                    cases=[make_case(shape) for shape in ("Gaussian", "Lorentzian", "PseudoVoigt", "Voigt")])
    output.write_text(json.dumps(artifact, separators=(",", ":"), allow_nan=False) + "\n")
    print(f"Wrote {output} ({output.stat().st_size:,} bytes)")
