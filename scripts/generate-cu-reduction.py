#!/usr/bin/env python3
"""Create a local, deterministic raw-mu copper reduction demonstration.

Requires rexafs 0.2.12 and NumPy. The checksum-matched Athena source is read
without modification. The destination must not exist. No random sampling,
normalization, energy alignment or added noise enters the generated mixtures.
Normalization below produces validation targets only, after mixing raw mu.
"""

import argparse
import base64
import csv
import gzip
import hashlib
import json
from pathlib import Path

import numpy as np
import rexafs
from rexafs.io import read_measurement

SOURCE_SHA256 = "18684eb2c4776d5d3661f6ad6fdfae6fb81d91571bda00611ec33bd686c67dc6"
SPECIES = ["CuO", "Cu2O", "Cu"]
SOURCE_LABELS = ["cuo_abs", "cu2o_abs", "cufoil_abs"]
SETTINGS = dict(e0=8979.0, pre_edge_start=-150.0, pre_edge_end=-75.0,
                norm_start=150.0, norm_end=650.0, norm_polyorder=2, n_victoreen=0)


def fractions(count=50):
    """Two overlapping smooth transitions; progress is dimensionless, not time."""
    progress = np.linspace(0.0, 1.0, count)

    def smoothstep(x):
        x = np.clip(x, 0.0, 1.0)
        return x * x * (3.0 - 2.0 * x)

    first = smoothstep(progress / 0.6)
    second = smoothstep((progress - 0.4) / 0.6)
    return progress, np.column_stack([1.0 - first, first - second, second])


def normalize(energy, mu):
    """Return the actual released rexafs preprocessing and measured edge step."""
    spectrum = rexafs.Spectrum(energy, mu)
    spectrum.set_normalization_method(rexafs.PrePostEdge(**SETTINGS)).normalize()
    norm = np.asarray(spectrum.norm())
    delta = mu - np.asarray(spectrum.pre_edge())
    # This identity reads the exact scale used, avoiding a nearest-E0 assumption.
    step = float(np.dot(delta, norm) / np.dot(norm, norm))
    return norm, np.asarray(spectrum.flat()), step


def generate(source, output):
    source_bytes = source.read_bytes()
    if hashlib.sha256(source_bytes).hexdigest() != SOURCE_SHA256:
        raise ValueError("The Athena source does not match the recorded measurement")
    if rexafs.__version__ != "0.2.12":
        raise ValueError("Use rexafs 0.2.12 for the recorded preparation")
    measurement = read_measurement(source)
    scans = measurement.document["scans"]
    originals = [measurement.arrays(scan=next(i for i, s in enumerate(scans)
                  if s["label"] == label)) for label in SOURCE_LABELS]
    low = max(x[0] for x, _ in originals)
    high = min(x[-1] for x, _ in originals)
    foil_grid = originals[2][0]
    energy = foil_grid[(foil_grid >= low) & (foil_grid <= high)]
    references = np.array([np.interp(energy, x, mu) for x, mu in originals])
    progress, weights = fractions()
    mixtures = weights @ references
    assert np.all(weights >= 0) and np.allclose(weights.sum(axis=1), 1)
    assert np.array_equal(mixtures[0], references[0])
    assert np.array_equal(mixtures[-1], references[2])
    # Validation only: never feed these arrays back into the raw-mu generator.
    prepared = [normalize(energy, mu) for mu in references]
    steps = np.array([p[2] for p in prepared])
    expected = weights * steps
    expected /= expected.sum(axis=1, keepdims=True)
    output.mkdir(parents=True, exist_ok=False)
    records, embedded, groups, labels, truth = [], {}, [], {}, []

    def retain(relative, content, kind="spectrum"):
        path = output / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        sha = hashlib.sha256(content).hexdigest()
        archive = "raw/" + relative.removeprefix("data/") if kind == "spectrum" else "analysis/" + Path(relative).name
        records.append(dict(path=relative, kind=kind, bytes=len(content), sha256=sha,
                            archive_path=archive))
        embedded[sha] = base64.b64encode(gzip.compress(content, mtime=0)).decode()

    def write_spectrum(relative, mu, label, metadata):
        header = ("# XDI/1.0 rexafs/0.2.12\n# Element.symbol: Cu\n# Element.edge: K\n"
                  "# Column.1: energy eV\n# Column.2: mu\n"
                  f"# Sample.name: {label}\n# Sample.description: {metadata}\n"
                  f"# Source.project_sha256: {SOURCE_SHA256}\n"
                  "# ///\n# ---\n# energy mu\n")
        content = header + "".join(f"{e:.17g} {y:.17g}\n" for e, y in zip(energy, mu))
        retain(relative, content.encode())
        group_id = "source:cu-reduction-v1:" + Path(relative).stem
        groups.append(dict(id=group_id, path=relative, channel="MuColumn"))
        labels[group_id] = label
        return group_id

    reference_ids = []
    for species, label, mu in zip(SPECIES, SOURCE_LABELS, references):
        reference_ids.append(write_spectrum(
            f"data/references/{species}.xdi", mu, f"Reference {species} · measured μ(E)",
            f"Measured {label}; raw absorption resampled linearly within measured overlap; no normalization."))
    mixture_ids = []
    for i, (t, mu, row, normalized) in enumerate(zip(progress, mixtures, weights, expected), 1):
        label = f"Synthetic {i:02d} · CuO {row[0]:.1%} · Cu₂O {row[1]:.1%} · Cu {row[2]:.1%}"
        relative = f"data/series/frame_{i:02d}.xdi"
        exact = "; ".join(f"{name}={value:.17g}" for name, value in zip(SPECIES, row))
        mixture_ids.append(write_spectrum(relative, mu, label,
            f"Synthetic raw-mu linear combination; raw weights {exact}; no added noise; frame is not elapsed time."))
        truth.append(dict(frame=i, progress=float(t), path=relative, label=label,
                          raw_mu_weights=row.tolist(), expected_normalized_weights=normalized.tolist()))
    csv_path = output / "fractions.csv"
    with csv_path.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["frame", "progress"] + [f"raw_mu_{s}" for s in SPECIES]
                        + [f"fixed_validation_normalized_{s}" for s in SPECIES])
        for row in truth:
            writer.writerow([row["frame"], row["progress"], *row["raw_mu_weights"],
                             *row["expected_normalized_weights"]])
    retain("fractions.csv", csv_path.read_bytes(), "analysis_artifact")
    manifest = dict(schema_version=1, synthetic=True, generation_space="raw mu(E)",
        species_order=SPECIES, frames=50, points=len(energy), added_noise=False,
        source=dict(filename=source.name, sha256=SOURCE_SHA256, labels=SOURCE_LABELS,
                    attribution="User-supplied group measurements; original project metadata retained separately"),
        versions=dict(rexafs=rexafs.__version__, numpy=np.__version__),
        energy_range_eV=[float(energy[0]), float(energy[-1])],
        interpolation="Linear onto foil samples within shared measured coverage; no extrapolation or energy shift",
        progression="t=(frame-1)/49; H(x)=clamp(x,0,1)^2*(3-2*clamp(x,0,1)); A=H(t/0.6); B=H((t-0.4)/0.6); weights=[1-A,A-B,B]",
        fraction_meaning="Coefficients multiplying unscaled measured raw mu; not calibrated mass or atomic fractions",
        desktop_processing="Standard automatic desktop defaults; no example-specific processing overrides",
        normalization_for_validation_only=SETTINGS, reference_edge_steps=steps.tolist(),
        normalized_fraction_rule="g_j=f_j*step_j/sum_k(f_k*step_k); inverse f_j=(g_j/step_j)/sum_k(g_k/step_k)",
        truth=truth)
    retain("manifest.json", (json.dumps(manifest, indent=2, ensure_ascii=False)+"\n").encode(), "analysis_artifact")
    project = dict(version=1, source_dir="data", spectrum_file="data/series/frame_01.xdi",
        # This explicit current default avoids the historical v1 clamp fallback.
        # All import, normalization, background and transform settings stay Auto.
        params={"bkg_clamp_policy": "FixedPenalty"},
        source_groups=groups, group_state=dict(labels=labels, marked=mixture_ids,
                                               current=mixture_ids[0]),
        synthetic_example=manifest, embedded=embedded,
        header=dict(format="rxs", format_version=1, software="rexafs local synthetic example",
                    software_version="0.2.12", created_utc="2026-09-21T00:00:00Z",
                    saved_utc="2026-09-21T00:00:00Z", storage="embedded",
                    path_base="project_directory", files=records))
    project_path = output / "Synthetic copper reduction.rxs"
    project_path.write_text(json.dumps(project, ensure_ascii=False)+"\n")
    print(json.dumps(dict(project=str(project_path), frames=50, references=3,
                         bytes=project_path.stat().st_size, reference_edge_steps=steps.tolist()), indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    generate(args.source, args.output)
