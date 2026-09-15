#!/usr/bin/env python3
"""Compare native copper recovery with NumPy and pyMCR, and plot the experiment.

Run after exporting the Rust integration-test results via REXAFS_CU_REPORT.
pyMCR is a development oracle only; the desktop does not depend on Python.
The custom three-component regressor enumerates simplex faces independently of
rexafs's active-set solver, rather than clipping/renormalizing unconstrained fits.
"""
import argparse
import itertools
import json
from importlib.metadata import version
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from pymcr.mcr import McrAR
from pymcr.regressors import LinearRegression


class SimplexRegression(LinearRegression):
    """Exact small convex-mixture least squares by enumerating all nonempty faces.

    A has shape energies × components, B energies × samples. Known row
    compositions, when supplied, replace only those independent sample solves.
    """
    def __init__(self, anchors=None):
        super().__init__()
        self.anchors = anchors or {}

    def fit(self, A, B):
        k, n = A.shape[1], B.shape[1]
        best = np.full(n, np.inf)
        answer = np.zeros((k, n))
        for count in range(1, k+1):
            for face in itertools.combinations(range(k), count):
                subset = A[:, face]
                matrix = np.block([[subset.T @ subset, np.ones((count,1))],
                                   [np.ones((1,count)), np.zeros((1,1))]])
                rhs = np.vstack([subset.T @ B, np.ones(n)])
                coefficients = np.linalg.solve(matrix, rhs)[:-1]
                valid = (coefficients >= -1e-12).all(axis=0)
                error = ((subset @ coefficients-B)**2).sum(axis=0)
                better = valid & (error < best)
                candidate = np.zeros((k,n)); candidate[list(face)] = coefficients
                answer[:,better] = candidate[:,better]
                best[better] = error[better]
        assert np.isfinite(best).all()
        for row, weights in self.anchors.items():
            answer[:,row] = weights
        self.X_ = answer


def matrix(serialized):
    values, rows, columns = serialized
    return np.array(values).reshape((rows, columns or 1), order="F")


def match_spectra(estimated, standards):
    """Match permutations only: closure fixes scale, so do not fit extra rescaling."""
    permutation = min(itertools.permutations(range(3)), key=lambda p:
                      np.linalg.norm(estimated[list(p)]-standards))
    estimate = estimated[list(permutation)]
    errors = np.linalg.norm(estimate-standards,axis=1)/np.linalg.norm(standards,axis=1)
    return list(permutation), errors


def analyze(fixtures, results, output):
    output.mkdir(parents=True,exist_ok=True)
    native = {p.stem:json.loads(p.read_text()) for p in results.glob("native-*.json")}
    labels = ["Cu foil", "Cu₂O", "CuO"]
    truth = np.array([r["weights"] for r in json.loads((fixtures/"truth.json").read_text())])
    records = json.loads((fixtures/"manifest.json").read_text())["standards"]
    arrays = [np.loadtxt(fixtures/r["path"]) for r in records]
    mask = (arrays[0][:,0]>=8950)&(arrays[0][:,0]<=9150)
    energy = arrays[0][mask,0]
    standards = np.array([a[mask,1] for a in arrays])
    data = np.array([np.loadtxt(fixtures/f"mixtures/mix_{i:03}.xdi")[mask,1] for i in range(1,101)])
    np.testing.assert_allclose(data,truth@standards,atol=1e-14,rtol=0)
    regressor = SimplexRegression(); regressor.fit(standards.T,data.T)
    reference_lcf = regressor.X_.T
    rust_lcf = np.array([[w["weight"] for w in row["weights"]] for row in native["native-lcf"]])
    np.testing.assert_allclose(rust_lcf, reference_lcf,atol=1e-8,rtol=0)
    summaries = dict(versions={p:version(p) for p in ["numpy","scipy","pymcr","rexafs","matplotlib"]},
                     spectra=100,points=int(mask.sum()),energy_range_eV=[float(energy[0]),float(energy[-1])],
                     lcf=dict(max_absolute_weight_error=float(np.max(np.abs(rust_lcf-truth))),
                              max_r_factor=max(r["r_factor"] for r in native["native-lcf"])),pca={},mcr={})
    for centered in [False,True]:
        d = data-data.mean(axis=0) if centered else data
        values = np.linalg.svd(d,compute_uv=False)**2
        fractions = values/values.sum()
        key = "centered" if centered else "uncentered"
        np.testing.assert_allclose(fractions,native["native-pca"][key]["variance_explained"],atol=1e-12,rtol=0)
        summaries["pca"][key] = fractions[:6].tolist()
    for key in ["blind-0","blind-71","anchored"]:
        rust = native[f"native-mcr-{key}"]
        d = data if key.startswith("blind") else np.vstack([data,standards])
        np.testing.assert_allclose(matrix(rust["data"]),d,rtol=0,atol=1e-14)
        anchors = {} if key.startswith("blind") else {100+i:np.eye(3)[i] for i in range(3)}
        initial = d[rust["initial_samples"]].copy()
        model = McrAR(c_regr=SimplexRegression(anchors),st_regr="OLS",c_constraints=[],st_constraints=[],
                      max_iter=2000,tol_increase=1e-6,tol_n_increase=100,
                      tol_n_above_min=100,tol_err_change=1e-26)
        model.fit(d,ST=initial)
        c, s = model.C_opt_,model.ST_opt_
        rust_s, rust_c = matrix(rust["spectra"]),matrix(rust["concentrations"])
        permutation, errors = match_spectra(rust_s,standards)
        residual = float(np.sum((d-c@s)**2)/np.sum(d*d))
        native_reference_difference = float(np.max(np.abs(rust_s-s)))
        assert residual < 1e-15
        assert native_reference_difference < 1e-5, (key,native_reference_difference)
        summaries["mcr"][key] = dict(native_relative_error=rust["relative_error"],
            native_iterations=rust["iterations"],native_termination=rust["termination"],
            reference_relative_error=residual,reference_iterations=int(model.n_iter),
            native_reference_max_spectral_difference=native_reference_difference,
            matched_component_indices=permutation,relative_spectral_errors=errors.tolist(),
            max_absolute_fraction_error=float(np.max(np.abs(rust_c[:100,permutation]-truth))))
        np.savez_compressed(output/f"reference-mcr-{key}.npz",concentrations=c,spectra=s,energy=energy)
    (output/"summary.json").write_text(json.dumps(summaries,indent=2)+"\n")

    plt.rcParams.update({"font.size":10,"axes.spines.top":False,"axes.spines.right":False})
    colors = ["#277da8","#d58222","#56875b"]
    fig, axes = plt.subplots(2,2,figsize=(12,8),layout="constrained")
    for i,label in enumerate(labels):
        axes[0,0].plot(energy,standards[i],label=label,color=colors[i])
        axes[0,1].scatter(truth[:,i],rust_lcf[:,i],s=14,alpha=.65,label=label,color=colors[i])
    axes[0,0].set(title="Prepared measured standards",xlabel="Energy (eV)",ylabel="Normalized absorption"); axes[0,0].legend()
    axes[0,1].plot([0,1],[0,1],"k--",lw=1)
    axes[0,1].set(title="Native LCF: all 100 mixtures",xlabel="Generating fraction",ylabel="Recovered coefficient")
    axes[0,1].legend()
    axes[1,0].plot(energy,data.T,alpha=.13,color="#547d9b",lw=.6)
    axes[1,0].set(title="100 random convex mixtures (no added noise)",xlabel="Energy (eV)",ylabel="Normalized absorption")
    for key,label in [("uncentered","Uncentered"),("centered","Mean subtracted")]:
        axes[1,1].semilogy(range(1,7),np.maximum(summaries["pca"][key],1e-32),"o-",label=label)
    axes[1,1].set(title="PCA: rank 3, or rank 2 after centering",xlabel="Direction number",ylabel="Squared-signal / variation fraction",xticks=range(1,7))
    axes[1,1].legend(); fig.savefig(output/"copper-lcf-pca.png",dpi=180); plt.close(fig)
    fig,axes = plt.subplots(1,3,figsize=(14,4.4),layout="constrained")
    for i,label in enumerate(labels):
        axes[i].plot(energy,standards[i],color="black",lw=2,label="Prepared original")
        for key,color,style in [("blind-0","#d58222","--"),("anchored","#277da8",":")]:
            permutation = summaries["mcr"][key]["matched_component_indices"]
            spectrum = matrix(native[f"native-mcr-{key}"]["spectra"])[permutation[i]]
            axes[i].plot(energy,spectrum,color=color,ls=style,lw=1.8,label="Blind MCR" if key.startswith("blind") else "Known pure samples")
        axes[i].set(title=label,xlabel="Energy (eV)",ylabel="Normalized absorption")
        axes[i].legend(fontsize=8)
    fig.suptitle("Native MCR: excellent reconstruction does not guarantee pure-component recovery")
    fig.savefig(output/"copper-mcr-recovery.png",dpi=180);plt.close(fig)
    print(json.dumps(summaries,indent=2))


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixtures",type=Path);parser.add_argument("results",type=Path);parser.add_argument("output",type=Path)
    args=parser.parse_args();analyze(args.fixtures,args.results,args.output)
