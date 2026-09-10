"""Record measured outputs using an installed binding; pass an output JSON path."""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import rexafs

ROOT = Path(__file__).resolve().parents[3]
FILES = {
    "cu": "xraylarch_d867/xafsdata/cu_150k.xmu",
    "ni": "xraylarch_d867/xafsdata/ni_metal_rt.xdi",
    "ru": "Ru_QAS.dat",
}
results = {}
for name, filename in FILES.items():
    path = ROOT / "crates/rexafs/tests/testfiles" / filename
    data = np.loadtxt(path)
    energy = data[:, 0]
    mu = np.log(data[:, 1] / data[:, 2]) if name == "ru" else data[:, 1]
    for policy, solver in [("FixedPenalty", "LinearDirect"), ("Fixed", "LegacyLm")]:
        background = rexafs.AUTOBK()
        background.clamp_scale_policy = policy
        background.solver = solver
        background.kmin = 0
        background.kmax = 12
        background.rbkg = 1
        background.kstep = 0.05
        background.nfft = 2048
        spectrum = (
            rexafs.Spectrum.from_arrays(energy, mu)
            .set_background_method(rexafs.BackgroundMethod.AUTOBK(background))
            .fft()
        )
        results[f"{name}-{policy}"] = {
            "input_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "e0": spectrum.e0(),
            "k": spectrum.k().tolist(),
            "chi": spectrum.chi().tolist(),
            "r": spectrum.r().tolist(),
            "chir_mag": spectrum.chir_mag().tolist(),
        }
Path(sys.argv[1]).write_text(json.dumps(results, separators=(",", ":")) + "\n")
print(f"Recorded {len(results)} measured pipelines using rexafs {rexafs.__version__}")
