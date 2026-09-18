#!/usr/bin/env python3
"""Generate an entirely synthetic RXS project for full-frame GUI qualification.

No experimental inputs are used. Energy is eV. Each signal is a linear baseline
plus a logistic unit edge and one Gaussian feature with a known varying height.
Frame 258 contains a narrow series transient. The 513 frames exceed the legacy
192-frame overview limit. Output is deliberately generated outside the checkout.
"""

import argparse
import json
import math
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="Destination .rxs file")
    args = parser.parse_args()
    energy = [9800.0 + 3.0 * i for i in range(401)]
    groups = []
    for index in range(513):
        amplitude = 0.12 + 0.04 * math.sin(index / 40.0)
        if index == 257:
            amplitude += 0.9
        mu = [
            0.15 + 0.00002 * (e - 10000.0)
            + 1.0 / (1.0 + math.exp(-(e - 10000.0) / 2.0))
            + amplitude * math.exp(-((e - 10025.0) / 9.0) ** 2)
            for e in energy
        ]
        groups.append({
            "id": index + 1, "label": f"Synthetic frame {index + 1:03d}",
            "energy": energy, "mu": mu,
            "quantity": "RawMu", "quantity_unconfirmed": False,
        })
    project = {
        "version": 1,
        "header": {
            "format": "rxs", "format_version": 1,
            "software": "rexafs synthetic validation generator",
            "software_version": "development",
            "created_utc": "2026-09-16T00:00:00Z",
            "saved_utc": "2026-09-16T00:00:00Z",
            "storage": "paths", "path_base": "project_directory", "files": [],
        },
        "params": {
            "e0": 10000.0, "pre_edge_start": -180.0, "pre_edge_end": -35.0,
            "norm_start": 150.0, "norm_end": 990.0, "norm_polyorder": 1,
        },
        "derived": groups, "active_derived": 1,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(project), encoding="utf-8")


if __name__ == "__main__":
    main()
