#!/usr/bin/env python3
"""Generate the explicitly authorized copper test collection from a local project.

Requires rexafs==0.2.8 and NumPy. The input is read-only; output must be a new
directory. No downloads occur. See doc/cu-mixture-recovery-plan.md for the model.
"""

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
import rexafs
from rexafs.io import read_measurement

SOURCE_SHA256 = "18684eb2c4776d5d3661f6ad6fdfae6fb81d91571bda00611ec33bd686c67dc6"
SEED = 20260916
SETTINGS = dict(e0=8979.0, pre_edge_start=-150.0, pre_edge_end=-75.0,
                norm_start=150.0, norm_end=650.0, norm_polyorder=2, n_victoreen=0)


def generate(source: Path, output: Path):
    """Normalize once, interpolate within overlap, and write 100 convex mixtures."""
    assert hashlib.sha256(source.read_bytes()).hexdigest() == SOURCE_SHA256, "Unexpected source"
    assert rexafs.__version__ == "0.2.8", "Use the recorded normalization version"
    measurement = read_measurement(source)
    scans = measurement.document["scans"]
    assert [s["label"] for s in scans] == ["cufoil_abs", "cu2o_abs", "cuo_abs"]
    originals = [measurement.arrays(scan=i) for i in range(3)]
    low = max(x[0] for x, _ in originals)
    high = min(x[-1] for x, _ in originals)
    grid = originals[0][0]
    grid = grid[(grid >= low) & (grid <= high)]
    spectra, records = [], []
    for scan, (x, mu) in zip(scans, originals):
        assert np.all(np.diff(x) > 0) and np.isfinite(mu).all()
        spectrum = rexafs.Spectrum(x, mu)
        spectrum.set_normalization_method(rexafs.PrePostEdge(**SETTINGS)).normalize()
        spectra.append(np.interp(grid, x, spectrum.norm()))
        records.append(dict(id=scan["id"], label=scan["label"], points=len(x),
                            energy_range_eV=[float(x[0]), float(x[-1])],
                            titles=[line for line in scan["header"].splitlines()
                                    if line.startswith("titles:")],
                            edge_step=float((spectrum.post_edge()-spectrum.pre_edge())[
                                np.abs(x-SETTINGS["e0"]).argmin()])))
    standards = np.array(spectra)
    weights = np.random.Generator(np.random.PCG64(SEED)).dirichlet(np.ones(3), size=100)
    mixtures = weights @ standards
    output.mkdir(parents=True, exist_ok=False)
    manifest = dict(schema_version=1, synthetic=True, noise="none added",
                    source=dict(filename=source.name, sha256=SOURCE_SHA256,
                                athena_version="0.8.061", created="2016-09-12 12:51:41",
                                acquisition_author="not established", source_url=None),
                    permission="User authorized derived test fixtures; upstream license not established.",
                    versions=dict(rexafs=rexafs.__version__, numpy=np.__version__),
                    seed=SEED, random_generator="NumPy PCG64", distribution="Dirichlet(1,1,1)",
                    normalization=SETTINGS, interpolation="linear; no extrapolation",
                    analysis_range_eV=[8950.0,9150.0], points=len(grid),
                    standards=records, files=[])

    def write_xdi(relative, values, label):
        path = output / relative
        path.parent.mkdir(exist_ok=True)
        header = ("# XDI/1.0 rexafs/0.2.8\n# Element.symbol: Cu\n# Element.edge: K\n# Column.1: energy eV\n# Column.2: mu\n"
                  f"# Sample.name: {label}\n# Sample.description: Derived test data; see manifest.json and README.md.\n"
                  "# ///\n# ---\n# energy mu\n")
        content = header + "".join(f"{x:.17g} {y:.17g}\n" for x,y in zip(grid,values))
        path.write_text(content, encoding="utf-8", newline="\n")
        manifest["files"].append(dict(path=relative, sha256=hashlib.sha256(content.encode()).hexdigest()))

    for record, values in zip(records, standards):
        record["path"] = f"standards/{record['label']}.xdi"
        write_xdi(record["path"], values, record["label"] + " prepared standard")
    truth = []
    for index, (values, fractions) in enumerate(zip(mixtures, weights), 1):
        relative = f"mixtures/mix_{index:03}.xdi"
        write_xdi(relative, values, f"Synthetic copper mixture {index:03}")
        truth.append(dict(path=relative, weights=fractions.tolist()))
    (output/"truth.json").write_text(json.dumps(truth, indent=2)+"\n")
    (output/"manifest.json").write_text(json.dumps(manifest, indent=2)+"\n")
    print(json.dumps(dict(mixtures=len(mixtures), points=len(grid),
                         grid_range_eV=[grid[0],grid[-1]], weights_min=weights.min(),
                         weights_max=weights.max()), indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    generate(args.source, args.output)
