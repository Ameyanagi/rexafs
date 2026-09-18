#!/usr/bin/env -S uv run --script
# /// script
# requires-python = "==3.12.*"
# dependencies = ["numpy==2.3.2", "scipy==1.16.1", "lmfit==1.3.4", "xraydb==4.5.8"]
# ///
"""Generate independent Larch comparisons from retained experimental spectra.

No rexafs output is used. The original measurement files are referenced by path
and SHA-256, not copied. Run with --output to a new directory when refreshing
references; existing historical artifacts are never overwritten. Numerical Larch
functions are loaded unchanged from a pinned revision, with only their interactive
decorators and group adapters replaced. See the fixture README for conventions.
"""
import argparse
import ast
import gzip
import hashlib
import io
import json
import platform
import sys
from pathlib import Path
from types import SimpleNamespace
from urllib.request import urlopen

import lmfit
import numpy as np
import scipy
import scipy.constants as constants
from scipy.fftpack import fft, ifft
from scipy.interpolate import UnivariateSpline, splev, splrep
from scipy.optimize import leastsq
from scipy.special import erfc, i0
import xraydb

ROOT = Path(__file__).resolve().parents[1]
XAS = ROOT / "crates/rexafs/tests/fixtures/xas"
COMMIT = "e3c93284fed358c2c8979cba4c139430527433c6"
BASE = f"https://raw.githubusercontent.com/xraypy/xraylarch/{COMMIT}/"
SOURCES = {}


def load(path, names, namespace):
    url = BASE + "larch/" + path
    source = urlopen(url, timeout=30).read()
    SOURCES[path] = dict(url=url, sha256=hashlib.sha256(source).hexdigest(), functions=names)
    tree = ast.parse(source)
    tree.body = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
    assert len(tree.body) == len(names), path
    for node in tree.body:
        node.decorator_list = []
    exec(compile(tree, url, "exec"), namespace)


def numerical_functions():
    ns = dict(np=np, erfc=erfc, MAX_NNORM=5, TINY_ENERGY=0.00050,
              sys=sys, Group=SimpleNamespace, NFEV=0,
              isgroup=lambda group, *attrs: all(hasattr(group, a) for a in attrs),
              parse_group_args=lambda x, members, defaults, group, fcn_name: (x, defaults[0], group),
              set_xafsGroup=lambda group, **kw: group,
              splrep=splrep, splev=splev, UnivariateSpline=UnivariateSpline, leastsq=leastsq,
              fft=fft, ifft=ifft, bessel_i0=i0, sqrtpi=np.sqrt(np.pi),
              ETOK=1 / (1.e20 * constants.hbar**2 / (2 * constants.m_e * constants.e)),
              FT_WINDOWS_SHORT=("kai", "han", "par", "wel", "gau", "sin", "fha"))
    for name in ("pi", "arange", "zeros", "ones", "sin", "cos", "exp", "log", "sqrt", "where", "interp", "linspace"):
        ns[name] = getattr(np, name)
    load("math/utils.py", ["index_of", "index_nearest", "remove_dups", "remove_nans", "remove_nans2", "polyfit", "realimag"], ns)
    load("xafs/pre_edge.py", ["preedge"], ns)
    load("xafs/mback.py", ["match_f2"], ns)
    load("xafs/xafsft.py", ["ftwindow", "xftf_fast"], ns)
    load("xafs/autobk.py", ["spline_eval", "_resid", "autobk"], ns)
    load("xafs/cauchy_wavelet.py", ["cauchy_wavelet"], ns)
    return ns


def raw_arrays(sample, kind):
    raw = (XAS / sample["path"]).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == sample["sha256"], sample["path"]
    text = raw.decode("utf-8")
    if kind == "9809":
        lines = text.splitlines()
        start = next(i for i, line in enumerate(lines) if line.strip().startswith("Offset")) + 1
        rows = np.loadtxt(io.StringIO("\n".join(lines[start:])))
        # Recorded observed angle and Si(311) spacing, with original I0/I1.
        energy = 12398.419843320026 / (2 * 1.63748 * np.sin(np.deg2rad(rows[:, 1])))
        mu = np.log(rows[:, 3] / rows[:, 4])
    else:
        rows = np.loadtxt(io.StringIO(text), comments="#")
        energy, mu = rows[:, 0], rows[:, 3 if kind == "xdi-rt" else 1]
    assert np.all(np.diff(energy) > 0) and np.all(np.isfinite(mu))
    return energy, mu


def mback(ns, energy, mu, element, e0, pre, post):
    f2 = xraydb.f2_chantler(element, energy)
    offset = energy - e0
    emission = xraydb.xray_line(element, "Ka1").energy
    params = lmfit.Parameters()
    params.add("s", value=2., min=1e-12)
    params.add("a", value=0., vary=False)
    params.add("xi", value=50., vary=False)
    for j in range(3):
        params.add(f"c{j}", value=0.)
    theta, weights = np.zeros_like(energy), np.ones_like(energy)
    for start, end in (pre, post):
        mask = (offset >= start) & (offset <= end)
        assert mask.sum() >= 3
        theta[mask], weights[mask] = 1., np.sqrt(mask.sum())
    fit = lmfit.minimize(ns["match_f2"], params, method="least_squares",
                        kws=dict(en=energy, mu=mu, f2=f2, e0=e0, em=emission, order=2, theta=theta, weight=weights),
                        x_scale="jac", max_nfev=5000, ftol=1e-13, xtol=1e-13, gtol=1e-13)
    assert fit.success, fit.message
    v = fit.params.valuesdict()
    background = sum(v[f"c{j}"] * offset**j for j in range(3))
    aux = ns["preedge"](energy, f2 + background, e0=e0, pre1=pre[0], pre2=pre[1],
                        norm1=post[0], norm2=post[1], nnorm=2, nvict=0)
    return dict(element=element, edge="K", e0=e0, pre_edge=pre, post_edge=post, degree=2, erfc=False,
                scale=v["s"], coefficients=[v[f"c{j}"] for j in range(3)],
                fit_indices=np.flatnonzero(theta), f2=f2, background=background,
                fpp=v["s"] * mu - background,
                norm=(v["s"] * mu - aux["pre_edge"]) / aux["edge_step"],
                edge_step=aux["edge_step"] / v["s"], pre_curve=aux["pre_edge"], post_curve=aux["post_edge"],
                objective=float(np.dot(fit.residual, fit.residual)))


def wavelet(ns, energy, mu, e0, pre, post):
    baseline = ns["preedge"](energy, mu, e0=e0, pre1=pre[0], pre2=pre[1],
                             norm1=post[0], norm2=post[1], nnorm=2, nvict=0)
    group = SimpleNamespace()
    preparation = dict(ek0=e0, edge_step=baseline["edge_step"], rbkg=1., kmin=0., kmax=14.,
                       kweight=1, dk=0.1, win="hanning", nfft=2048, kstep=0.05,
                       nclamp=3, clamp_lo=0., clamp_hi=1., calc_uncertainties=False)
    ns["autobk"](energy.copy(), mu.copy(), group, **preparation)
    assert np.all(np.isfinite(group.chi)) and len(group.k) == 281
    output = SimpleNamespace()
    ns["cauchy_wavelet"](group.k, group.chi, output, kweight=2, rmax_out=4., nfft=1024)
    return dict(preparation=preparation, pre_edge=pre, post_edge=post,
                k=group.k, chi=group.chi, kweight=2, kstep=0.05, order=len(output.wcauchy_r),
                larch_nfft=1024, actual_fft_length=2048, r=output.wcauchy_r,
                real=output.wcauchy_re.ravel(), imaginary=output.wcauchy_im.ravel())


def encode(value):
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, np.generic):
        return value.item()
    raise TypeError(type(value).__name__)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "crates/rexafs/tests/fixtures/analysis/experimental-larch")
    out = parser.parse_args().output
    out.mkdir(parents=True, exist_ok=False)
    samples = {s["id"]: s for s in json.loads((XAS / "manifest.json").read_text())["samples"]}
    ns = numerical_functions()
    cases = [
        ("cu-rt", "aps-13-id-c-xasdatalibrary-cu-metal-rt-xdi", "xdi-rt", "Cu", 8980., [-150., -40.]),
        ("cu-10k", "nsls-x11a-xasdatalibrary-cu-metal-10k-xdi", "xdi-10k", "Cu", 8980., [-150., -40.]),
        ("ruo2", "mdr-b8ade3fa-ruo2-f-aichisr-bl11s-20200903-dat", "9809", "Ru", 22140., [-250., -50.]),
    ]
    records = []
    for name, sample_id, kind, element, e0, pre in cases:
        sample = samples[sample_id]
        energy, mu = raw_arrays(sample, kind)
        data = dict(energy=energy, mu=mu, mback=mback(ns, energy, mu, element, e0, pre, [100., 800.]),
                    wavelet=wavelet(ns, energy, mu, e0, pre, [100., 800.]))
        payload = gzip.compress(json.dumps(data, default=encode, separators=(",", ":"), allow_nan=False).encode() + b"\n", mtime=0)
        filename = name + ".json.gz"
        (out / filename).write_bytes(payload)
        attribution = (XAS / (sample["path"] + ".license")).read_text()
        (out / (filename + ".license")).write_text(
            "This file contains numerical reference arrays derived from the following original experimental measurement.\n"
            "Original data attribution and terms follow; the source data were not modified.\n\n" + attribution)
        records.append(dict(id=name, fixture=filename, sha256=hashlib.sha256(payload).hexdigest(),
                            sample=sample, conversion=kind, points=len(energy)))
        print(f"{name}: {len(energy)} experimental rows; {len(data['wavelet']['r'])} x 281 complex wavelet cells", flush=True)
    manifest = dict(schema_version=1, kind="experimental", generator=Path(__file__).relative_to(ROOT).as_posix(),
                    larch_commit=COMMIT, sources=SOURCES, python=platform.python_version(),
                    numpy=np.__version__, scipy=scipy.__version__, lmfit=lmfit.__version__,
                    xraydb=xraydb.__version__, database=xraydb.get_xraydb().get_version(),
                    tolerances=dict(mback_curve_absolute=1e-6, mback_post_curve_absolute=5e-6,
                                    mback_scale_absolute=1e-7,
                                    mback_objective_absolute=1e-9, wavelet_absolute=2e-10), cases=records)
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2, allow_nan=False) + "\n")
    (out / "LARCH-LICENSE.txt").write_bytes(urlopen(BASE + "LICENSE", timeout=30).read())


if __name__ == "__main__":
    main()
