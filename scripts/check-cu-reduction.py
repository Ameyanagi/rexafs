#!/usr/bin/env python3
"""Validate the local raw-mu example; report recovery separately from truth.

Use the environment documented by generate-cu-reduction.py plus matplotlib and
pyMCR. No input file is modified. PCA/LCF use NumPy; MCR uses pyMCR with an
independent simplex solver. These are reference checks, not desktop GUI tests.
"""

import argparse
import importlib.util
import itertools
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from pymcr.mcr import McrAR


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


generator = module("generator", Path(__file__).with_name("generate-cu-reduction.py"))
reference = module("reference", Path(__file__).with_name("check-cu-mixtures-reference.py"))


def check(root):
    manifest = json.loads((root / "manifest.json").read_text())
    truth = np.array([r["raw_mu_weights"] for r in manifest["truth"]])
    expected = np.array([r["expected_normalized_weights"] for r in manifest["truth"]])
    refs = [np.loadtxt(root / f"data/references/{s}.xdi") for s in generator.SPECIES]
    energy = refs[0][:, 0]
    raw_refs = np.array([r[:, 1] for r in refs])
    raw = np.array([np.loadtxt(root / r["path"])[:, 1] for r in manifest["truth"]])
    np.testing.assert_allclose(raw, truth @ raw_refs, atol=2e-15, rtol=0)
    np.testing.assert_array_equal(raw[0], raw_refs[0])
    np.testing.assert_array_equal(raw[-1], raw_refs[2])
    np.testing.assert_array_equal(truth, generator.fractions()[1])
    assert (np.diff(truth[:, 0]) <= 0).all() and (np.diff(truth[:, 2]) >= 0).all()
    assert (truth > 0).all(axis=1).sum() >= 8
    mask = (energy >= 8950) & (energy <= 9150)
    normalized_refs = [generator.normalize(energy, mu) for mu in raw_refs]
    normalized_data = [generator.normalize(energy, mu) for mu in raw]
    summary = {"frames": 50, "references": 3, "generation": "deterministic raw mu; no random sampling or added noise", "spaces": {}}
    matrices = {"raw": (raw_refs[:, mask], raw[:, mask], truth)}
    for i, space in enumerate(["norm", "flat"]):
        matrices[space] = (np.array([r[i][mask] for r in normalized_refs]),
                           np.array([r[i][mask] for r in normalized_data]), expected)
    lcf_results = {}
    for space, (standards, data, weights) in matrices.items():
        fit = reference.SimplexRegression()
        fit.fit(standards.T, data.T)
        estimated = fit.X_.T
        lcf_results[space] = estimated
        err = float(np.max(np.abs(estimated - weights)))
        assert err < 1e-8, (space, err)
        centered = data - data.mean(axis=0)
        squared = np.linalg.svd(centered, compute_uv=False) ** 2
        variance = squared / squared.sum()
        assert variance[1] > 1e-4 and variance[2:].sum() < 1e-20
        summary["spaces"][space] = dict(lcf_max_absolute_fraction_error=err,
            mixture_identity_max_error=float(np.max(np.abs(data - weights @ standards))),
            centered_pca_variance=variance[:4].tolist(), centered_pca_directions=2)
    steps = np.array(manifest["reference_edge_steps"])
    corrected = lcf_results["flat"] / steps
    corrected /= corrected.sum(axis=1, keepdims=True)
    summary["inverse_edge_step_max_raw_fraction_error"] = float(np.max(np.abs(corrected-truth)))
    summary["max_normalized_vs_raw_fraction_difference"] = float(np.max(np.abs(expected-truth)))
    # Blind initialization uses observed rows only; references/truth stay out.
    standards, data, expected = matrices["flat"]
    first = int(np.argmax(np.linalg.norm(data-data.mean(axis=0), axis=1)))
    second = int(np.argmax(np.linalg.norm(data-data[first], axis=1)))
    axis = data[second] - data[first]
    projection = data[first] + np.outer((data-data[first]) @ axis / (axis @ axis), axis)
    third = int(np.argmax(np.linalg.norm(data-projection, axis=1)))
    initial = [first, second, third]
    mcr = McrAR(c_regr=reference.SimplexRegression(), st_regr="OLS",
        c_constraints=[], st_constraints=[], max_iter=2000, tol_increase=1e-6,
        tol_n_increase=100, tol_n_above_min=100, tol_err_change=1e-26)
    mcr.fit(data, ST=data[initial].copy())
    permutation = min(itertools.permutations(range(3)), key=lambda p:
        np.linalg.norm(mcr.ST_opt_[list(p)] - standards))
    resolved = mcr.ST_opt_[list(permutation)]
    coefficients = mcr.C_opt_[:, list(permutation)]
    residual = float(np.sum((data-mcr.C_opt_@mcr.ST_opt_)**2) / np.sum(data**2))
    assert residual < 1e-12
    summary["blind_mcr_flat"] = dict(iterations=int(mcr.n_iter),
        initialization_frames=[i+1 for i in initial], relative_squared_residual=residual,
        matched_spectral_relative_errors=(np.linalg.norm(resolved-standards,axis=1)/np.linalg.norm(standards,axis=1)).tolist(),
        max_normalized_fraction_error=float(np.max(np.abs(coefficients-expected))),
        note="Blind MCR reconstruction is not proof of unique pure-reference or fraction recovery")
    (root / "validation.json").write_text(json.dumps(summary, indent=2)+"\n")
    colors = ["#bc4b35", "#de9a27", "#277aab"]
    fig, axes = plt.subplots(2, 2, figsize=(12, 8), layout="constrained")
    frames = np.arange(1, 51)
    for i, (label, color) in enumerate(zip(["CuO", "Cu₂O", "Cu"], colors)):
        axes[0, 0].plot(frames, 100*truth[:,i], color=color, label=label)
        axes[1, 0].plot(frames, 100*expected[:,i], color=color, label=label+" expected")
        axes[1, 0].scatter(frames[::3], 100*lcf_results['flat'][::3,i], color=color, s=13)
        axes[1, 1].plot(frames, 100*expected[:,i], color=color, label=label+" expected")
        axes[1, 1].plot(frames, 100*coefficients[:,i], color=color, ls="--")
    for i in range(50):
        axes[0, 1].plot(energy[mask],raw[i,mask],color=plt.cm.viridis(i/49),alpha=.7,lw=.8)
    axes[0,0].set(title="Known raw μ mixing fractions",xlabel="Synthetic frame",ylabel="Raw μ coefficient (%)")
    axes[0,1].set(title="50 systematically generated raw μ spectra",xlabel="Energy (eV)",ylabel="Absorption (source units)")
    axes[1,0].set(title="Normalized LCF: expected lines, fitted points",xlabel="Synthetic frame",ylabel="Edge-step-weighted coefficient (%)")
    axes[1,1].set(title="Blind MCR: dashed estimates, solid truth",xlabel="Synthetic frame",ylabel="Edge-step-weighted coefficient (%)")
    for ax in [axes[0,0],axes[1,0],axes[1,1]]:
        ax.legend(fontsize=8);ax.grid(alpha=.2)
    fig.suptitle("Synthetic CuO → Cu₂O → Cu · measured raw μ references · no added noise")
    fig.savefig(root/"validation.png",dpi=160)
    plt.close(fig)
    print(json.dumps(summary,indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    check(parser.parse_args().root)
