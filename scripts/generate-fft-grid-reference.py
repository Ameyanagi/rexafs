#!/usr/bin/env python3
"""Retain independent XrayLarch FFT endpoint references (requires NumPy/SciPy).

Only the named numerical functions from an immutable upstream source are loaded;
no rexafs code or output is used to calculate the reference arrays.
"""
import ast
import hashlib
import json
from pathlib import Path
from urllib.request import urlopen

import numpy as np
import scipy
from scipy.special import i0

SOURCE = "https://raw.githubusercontent.com/xraypy/xraylarch/860d8a690c81eefb0e61dee4ca3703ef4b67e93d/larch/xafs/xafsft.py"
source = urlopen(SOURCE, timeout=30).read()
namespace = {name: getattr(np, name) for name in (
    "pi", "arange", "zeros", "ones", "sin", "cos", "exp", "log", "sqrt", "where", "interp", "linspace"
)}
namespace.update(np=np, bessel_i0=i0, FT_WINDOWS_SHORT=["han", "fha", "par", "wel", "gau", "sin", "kai", "bes"])
tree = ast.parse(source)
tree.body = [node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name in ("ftwindow", "xftf_prep")]
assert len(tree.body) == 2
exec(compile(tree, SOURCE, "exec"), namespace)

cases = []
for window, larch_name in (("KaiserBessel", "kaiser"), ("Hanning", "hanning"), ("Parzen", "parzen"), ("Welch", "welch"), ("Gaussian", "gaussian"), ("Sine", "sine"), ("FHanning", "fhanning")):
    for end in (11.75, 12.0, 13.25):
        for irregular in (False, True):
            k = np.arange(0, end + 0.001, 0.05)
            if irregular:
                k = k[7:]
                k[1:-1] += 0.006 * np.sin(np.arange(1, len(k)-1))
            basis = np.array([np.sin(4.4*k), np.cos(4.4*k)])
            chi = 0.7*basis[0] + 0.2*basis[1] + 0.03*np.sin(6.1*k)
            args = dict(kmin=2, kmax=12, kweight=2, dk=1, dk2=1.5, window=larch_name, nfft=512, kstep=0.05)
            def transform(values):
                weighted, win = namespace["xftf_prep"](k, values, **args)
                return np.fft.rfft(weighted*win, n=512)*0.05/np.sqrt(np.pi), win
            chir, win = transform(chi)
            basis_fft = np.array([transform(values)[0] for values in basis]).T
            # Fit amplitude and phase at a fixed shell distance using complex
            # R-space samples, including an off-shell contaminating signal.
            r = np.arange(len(chir))*np.pi/(0.05*512)
            selected = (r >= 1) & (r <= 3)
            matrix = np.concatenate((basis_fft[selected].real, basis_fft[selected].imag))
            data = np.concatenate((chir[selected].real, chir[selected].imag))
            fitted = np.linalg.lstsq(matrix, data, rcond=None)[0]
            cases.append(dict(name=f"{window}-{end}-{'irregular' if irregular else 'uniform'}", window=window, k=k.tolist(), chi=chi.tolist(), kwin=win.tolist(), real=chir.real.tolist(), imag=chir.imag.tolist(), shell_amplitude=float(np.linalg.norm(fitted)), shell_phase=float(np.arctan2(fitted[1], fitted[0]))))

output = Path(__file__).resolve().parents[1]/"crates/rexafs/tests/testfiles/fft_grid_larch_reference.json"
output.write_text(json.dumps(dict(source=SOURCE, source_sha256=hashlib.sha256(source).hexdigest(), numpy=np.__version__, scipy=scipy.__version__, cases=cases), separators=(",", ":"))+"\n")
print(f"Wrote {len(cases)} cases, {output.stat().st_size} bytes: {output}")
