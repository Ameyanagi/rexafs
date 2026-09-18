#!/usr/bin/env python3
"""Plot a local experimental RMC run; requires NumPy and Matplotlib.

Reads the atomic checkpoint, never the measured project. Output is a PNG, SVG,
PDF, CSV and a local HTML viewer. The best encountered state is labeled by
its attempted-move count; completion of a budget does not mean convergence.
The last accepted state is also exported. No data are transmitted.
Fourier distance is not corrected for scattering phase. This script's negative
FFT uses the same 0.05/sqrt(pi) scaling as rexafs XrayFFTF; if curves.json from
the Rust example exists, the arrays are checked against it before plotting.
Use --watch to refresh the local PNG until summary.json marks completion.
"""
import argparse
import json
import time
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np


def transform(k, chi):
    """k²-weighted Hanning transform, 1 Å⁻¹ taper inside each support boundary."""
    dk, nfft = 0.05, 2048
    grid = np.arange(round(k[-1] / dk) + 1) * dk
    padded = np.zeros(len(grid))
    indices = np.rint(k / dk).astype(int)
    np.testing.assert_allclose(grid[indices], k, atol=1e-10, rtol=0)
    padded[indices] = chi
    window = np.zeros(len(grid))
    window[(grid >= k[0]) & (grid <= k[-1])] = 1
    lower = (grid >= k[0]) & (grid <= k[0] + 1)
    upper = (grid >= k[-1] - 1) & (grid <= k[-1])
    window[lower] = np.sin(np.pi / 2 * (grid[lower] - k[0])) ** 2
    window[upper] = np.cos(np.pi / 2 * (grid[upper] - (k[-1] - 1))) ** 2
    result = np.fft.rfft(padded * grid**2 * window, n=nfft) * dk / np.sqrt(np.pi)
    r = np.arange(len(result)) * np.pi / (nfft * dk)
    return r[r <= 6], result[r <= 6]


def render(directory):
    checkpoint = json.loads((directory / "checkpoint.json").read_text())
    dataset = checkpoint["problem"]["datasets"][0]["exafs"]
    if dataset["kweight"] != 2:
        raise ValueError("This Cu2O plotting example requires kweight=2.")
    k = np.array(dataset["k"])
    complete = (directory / "summary.json").exists()
    step = checkpoint["completed"]
    convergence = json.loads((directory / "convergence.json").read_text()) if (directory / "convergence.json").exists() else None
    trend_labels = {"InsufficientHistory": "Insufficient history for plateau assessment", "StillChanging": "Residual still changing — convergence not established", "ResidualPlateau": "Numerical residual plateau detected — structural convergence not established"}
    trend_label = trend_labels.get(convergence["status"], "Convergence not assessed") if convergence else "Convergence not assessed"
    states = {"initial": checkpoint["initial"], "best": checkpoint["best"],
              "final": checkpoint["current"]}
    curves = {"experimental": np.array(dataset["chi"])}
    curves.update({name: np.array(state["evaluation"]["datasets"][0]["chi"])
                   for name, state in states.items()})
    fourier = {name: transform(k, chi) for name, chi in curves.items()}
    if (directory / "curves.json").exists():
        rust = json.loads((directory / "curves.json").read_text())
        for name, (r, values) in fourier.items():
            reference = rust["spectra"][name]
            np.testing.assert_allclose(r, reference["r"][:len(r)], atol=1e-12)
            np.testing.assert_allclose(values.real, reference["real"][:len(r)], atol=1e-10)
            np.testing.assert_allclose(values.imag, reference["imag"][:len(r)], atol=1e-10)
    initial_score = states["initial"]["evaluation"]["score"]
    best_score = states["best"]["evaluation"]["score"]
    improvement = 100 * (1 - best_score / initial_score)
    labels = {"experimental": "Experiment · cu2o_abs", "initial": "Initial crystal fit",
              "best": f"Best RMC fit · {step:,} trials"}
    colors = {"experimental": "#212b36", "initial": "#5285b8", "best": "#dc5939"}
    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 11,
                         "axes.spines.top": False, "axes.spines.right": False,
                         "axes.labelcolor": "#29384a", "axes.titleweight": "bold",
                         "figure.facecolor": "white", "axes.axisbelow": True,
                         "savefig.facecolor": "white"})
    fig = plt.figure(figsize=(13, 8.3), layout="constrained")
    gs = fig.add_gridspec(2, 2, height_ratios=[1, 1])
    ak = fig.add_subplot(gs[0, :]); ar = fig.add_subplot(gs[1, 0]); ap = fig.add_subplot(gs[1, 1])
    for name in ["initial", "experimental", "best"]:
        style = dict(color=colors[name], lw=1.7, label=labels[name],
                     ls="--" if name == "initial" else "-", alpha=.88 if name == "initial" else 1)
        ak.plot(k, k**2 * curves[name], **style)
        r, values = fourier[name]
        ar.plot(r, np.abs(values), **style)
        ap.plot(r, values.real, **style)
    ak.set(xlabel=r"$k$ (Å$^{-1}$)", ylabel=r"$k^2\chi(k)$ (Å$^{-2}$)", xlim=(k[0], k[-1]), title="k space")
    ar.set(xlabel=r"$R$ (Å; no phase correction)", ylabel=r"$|\chi(R)|$ (Å$^{-3}$)", xlim=(0, 6), title="R space · magnitude")
    ap.set(xlabel=r"$R$ (Å; no phase correction)", ylabel=r"Re $\chi(R)$ (Å$^{-3}$)", xlim=(0, 6), title="R space · real component")
    for ax in [ak, ar, ap]: ax.grid(alpha=.15)
    ak.legend(loc="upper left", ncols=3, fontsize=10, framealpha=.93)
    title = "Cu₂O · experimental reverse Monte Carlo" + ("" if complete else " · running")
    fig.suptitle(f"{title}\n48 atoms · all 32 Cu absorbers · {step:,} trials\n{trend_label}", fontsize=14)
    fig.supxlabel("Common R transform: k² weighting, 2.5–12 Å⁻¹ support, 1 Å⁻¹ Hanning end tapers.\nReFEFF potentials fixed to the initial crystal; no added Debye–Waller damping.", fontsize=9, color="#526170")
    fig.savefig(directory / "cu2o-k-r.png", dpi=160)
    if complete:
        fig.savefig(directory / "cu2o-k-r.pdf")
        fig.savefig(directory / "cu2o-k-r.svg")
    plt.close(fig)

    if complete:
        fig, detail = plt.subplots(2, 2, figsize=(13, 7), layout="constrained",
                                   gridspec_kw={"height_ratios": [2, 1]})
        for name in ["experimental", "best"]:
            r, values = fourier[name]
            detail[0, 0].plot(k, k**2 * curves[name], color=colors[name], label=labels[name])
            detail[0, 1].plot(r, np.abs(values), color=colors[name], label=labels[name])
        r, model_r = fourier["best"]
        difference_r = model_r - fourier["experimental"][1]
        detail[1, 0].plot(k, k**2 * (curves["best"] - curves["experimental"]), color="#736a87")
        detail[1, 1].plot(r, difference_r.real, color="#736a87", label="Real residual")
        detail[1, 1].plot(r, difference_r.imag, color="#736a87", ls="--", label="Imaginary residual")
        detail[0, 0].set(ylabel=r"$k^2\chi(k)$ (Å$^{-2}$)", title=f"Best after {step:,} trials · k space", xlim=(k[0], k[-1]))
        detail[0, 1].set(ylabel=r"$|\chi(R)|$ (Å$^{-3}$)", title=f"Best after {step:,} trials · R magnitude", xlim=(0, 6))
        detail[1, 0].set(xlabel=r"$k$ (Å$^{-1}$)", ylabel=r"Fit − experiment (Å$^{-2}$)", xlim=(k[0], k[-1]))
        detail[1, 1].set(xlabel=r"$R$ (Å; no phase correction)", ylabel=r"Fit − experiment (Å$^{-3}$)", xlim=(0, 6))
        for ax in detail.flat: ax.grid(alpha=.15)
        for ax in detail[0]: ax.legend(fontsize=9)
        detail[1, 1].legend(fontsize=9)
        fig.suptitle(f"Cu₂O · best fit after {step:,} trials\n{trend_label}", fontsize=14)
        fig.savefig(directory / "cu2o-final-detail.png", dpi=160)
        fig.savefig(directory / "cu2o-final-detail.pdf")
        plt.close(fig)

    history = checkpoint["history"]
    accepted = sum(row["accepted"] for row in history)
    previous = initial_score
    uphill = []
    for row in history:
        if row["accepted"] and row["score"] > previous: uphill.append(row)
        previous = row["score"]
    fig, axes = plt.subplots(1, 2, figsize=(13, 4.3), layout="constrained")
    steps = [0] + [row["step"] for row in history]
    axes[0].semilogy(steps, [initial_score] + [row["score"] for row in history], color="#789bb2", label="Accepted chain")
    axes[0].semilogy(steps, [initial_score] + [row["best_score"] for row in history], color=colors["best"], label="Best encountered")
    if uphill:
        axes[0].scatter([row["step"] for row in uphill], [row["score"] for row in uphill], s=15, color="#4c866b", zorder=3, label=f"Uphill acceptance ({len(uphill)})")
    axes[0].set(xlabel="Attempted single-atom moves", ylabel="Mean squared k² residual", title=f"{accepted} accepted · {len(uphill)} uphill")
    axes[0].legend(fontsize=9); axes[0].grid(alpha=.15)
    # Minimum-image Cu–O first shell; cutoff below half the orthogonal box edge.
    for name in ["initial", "best"]:
        config = states[name]["structures"][0]["configuration"]
        positions = np.array([a["position"] for a in config["atoms"]])
        z = np.array([a["atomic_number"] for a in config["atoms"]])
        cell = np.array(config["cell"])
        np.testing.assert_allclose(cell, np.diag(np.diag(cell)), atol=1e-12)
        delta = positions[z == 29, None, :] - positions[z == 8, :]
        delta -= np.rint(delta / np.diag(cell)) * np.diag(cell)
        distances = np.linalg.norm(delta, axis=-1).ravel()
        first = distances[distances < 2.4]
        counts, edges = np.histogram(first, bins=np.arange(1.4, 2.421, .02))
        axes[1].stairs(counts / np.sum(z == 29), edges, color=colors[name], lw=1.7,
                        label=f"{name.capitalize()} · mean {first.mean():.3f} Å")
    axes[1].set(xlabel="Cu–O distance (Å)", ylabel="Neighbors per Cu per 0.02 Å bin", title="Coordinate-derived first-shell distribution", xlim=(1.4, 2.4))
    axes[1].legend(fontsize=9); axes[1].grid(alpha=.15)
    fig.savefig(directory / "cu2o-diagnostics.png", dpi=160); plt.close(fig)
    np.savetxt(directory / "k-space.csv", np.column_stack([k, *curves.values()]), delimiter=",", header="k_A^-1," + ",".join(curves), comments="")
    r = fourier["experimental"][0]
    np.savetxt(directory / "r-space.csv", np.column_stack([r, *[v for _, c in fourier.values() for v in (c.real, c.imag, np.abs(c))]]), delimiter=",",
               header="R_A," + ",".join(f"{name}_{part}" for name in fourier for part in ["real", "imag", "magnitude"]), comments="")
    last_ratio = states["final"]["evaluation"]["score"] / initial_score
    metadata = {"completed":complete,"steps":step,"accepted":accepted,"uphill_accepted":len(uphill),
                "initial_score":initial_score,"best_score":best_score,"improvement_percent":improvement,
                "best_k_rfactor":sum((k**2*(curves["best"]-curves["experimental"]))**2)/sum((k**2*curves["experimental"])**2),
                "last_relative_to_initial":last_ratio,"convergence":convergence}
    (directory / "plot-summary.json").write_text(json.dumps(metadata, indent=2))
    refresh = '<meta http-equiv="refresh" content="20">' if not complete else ""
    status = "Attempt budget completed; convergence not established" if complete else "Running; this page refreshes every 20 seconds"
    detail_image = '<img src="cu2o-final-detail.png" alt="Best fit at experimental amplitude scale, with k and complex R residuals">' if complete else ""
    document = f'''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">{refresh}
<title>Cu₂O RMC · experiment and best fit after {step:,} trials</title><style>body{{margin:24px auto;max-width:1250px;font:16px system-ui;color:#243345;background:#f2f5f8;padding:0 18px}}img{{width:100%;height:auto;background:white;border-radius:8px}}p{{line-height:1.55}}a{{color:#135997}}.card{{background:white;padding:16px 22px;border-radius:8px;margin:16px 0}}h1{{font-size:25px}}</style>
<h1>Cu₂O · measured data and reverse Monte Carlo</h1><p>{status} · {step:,} trials · {accepted} accepted · {len(uphill)} uphill</p><p><strong>{trend_label}</strong></p>
<img src="cu2o-k-r.png?{step}" alt="Experimental, initial crystal and best RMC spectra in k space and complex R space">
<div class="card"><b>What is compared</b><p>The black curve is the Cu K-edge measurement in your <code>Cu oxides.prj</code>, group <code>cu2o_abs</code>, reprocessed in REXAFS. Blue is the initial perfect crystal. Orange is the best configuration encountered {"in this completed run" if complete else "so far"}. Both fit curves use the same fixed S₀²={dataset['s02']:.2f} and ΔE₀={dataset['delta_e0']:.3f} eV.</p>
<p>The numerical k² residual fell from {initial_score:.6f} to {best_score:.6f} ({improvement:.1f}%). The experimental-normalized squared residual is {metadata['best_k_rfactor']:.4f}. This finite run is an experimental refinement, with fixed lattice and reference potentials. It does not establish a unique structure or sampling convergence. R-space peaks include scattering phase; their positions are not direct bond lengths.</p></div>
{detail_image}
<img src="cu2o-diagnostics.png?{step}" alt="Monte Carlo score history with uphill acceptances, and Cu-O distance distributions">
<p><a href="cu2o-k-r.pdf">PDF</a> · <a href="cu2o-k-r.svg">SVG</a> · <a href="k-space.csv">k-space data</a> · <a href="r-space.csv">R-space data</a> · <a href="summary.json">Run summary</a></p></html>'''
    (directory / "plots.html").write_text(document)
    print(json.dumps(metadata), flush=True)
    return complete


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--watch", action="store_true")
    args = parser.parse_args()
    while True:
        if render(args.directory) or not args.watch: break
        time.sleep(20)
