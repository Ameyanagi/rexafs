#!/usr/bin/env python3
"""Write synthetic completed or incremental XDI scans for Live GUI qualification.

No experimental data are read. Energy is eV; the signal is a linear baseline,
logistic edge and varying Gaussian peak. Files are created exclusively: an
existing source is never overwritten. Output must stay outside the repository.
"""
import argparse
import math
from pathlib import Path
import time


def spectrum(index: int, three_signals: bool = False) -> tuple[str, list[str]]:
    energy = [9800.0 + 3.0 * i for i in range(401)]
    height = 0.12 + 0.06 * math.sin(index / 3.0) + (0.5 if index == 7 else 0.0)
    values = [0.15 + 0.00002 * (e - 10000.0)
              + 1.0 / (1.0 + math.exp(-(e - 10000.0) / 2.0))
              + height * math.exp(-((e - 10025.0) / 9.0) ** 2) for e in energy]
    if three_signals:
        # These synthetic detector counts encode three distinct absorption curves.
        # They are not measured intensities or a calibrated detector simulation.
        header = ("# XDI/1.0 rexafs/live-test\n# Column.1: energy eV\n"
                  "# Column.2: i0 counts\n# Column.3: it counts\n"
                  "# Column.4: if counts\n# Column.5: ir counts\n# ///\n"
                  "# Entirely synthetic three-channel software test\n"
                  "# -----\n# energy i0 it if ir\n")
        rows = []
        for e, y in zip(energy, values):
            i0 = 100000.0
            it = i0 * math.exp(-y)
            fluorescence = i0 * (0.3 * y + 0.02)
            ir = it * math.exp(-(0.8 * y + 0.1))
            rows.append(f"{e} {i0} {it:.12g} {fluorescence:.12g} {ir:.12g}\n")
        return header, rows
    header = ("# XDI/1.0 rexafs/live-test\n# Column.1: energy eV\n"
              "# Column.2: mu\n# ///\n# Entirely synthetic software-test signal\n"
              "# -----\n# energy mu\n")
    return header, [f"{e} {y}\n" for e, y in zip(energy, values)]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--three-signals", action="store_true",
                        help="Write synthetic transmission, fluorescence and reference counts")
    parser.add_argument("--start", type=int, default=1)
    parser.add_argument("--count", type=int, default=1)
    parser.add_argument("--interval", type=float, default=0.5)
    parser.add_argument("--partial-pause", type=float, default=0.3)
    args = parser.parse_args()
    if args.start < 1 or args.count < 1 or args.interval < 0 or args.partial_pause < 0:
        parser.error("Start/count must be positive; delays must be nonnegative")
    root = Path(__file__).resolve().parents[1]
    output = args.directory.resolve()
    if output == root or root in output.parents:
        parser.error("Use a temporary output directory outside the checkout")
    output.mkdir(parents=True, exist_ok=True)
    for index in range(args.start, args.start + args.count):
        path = output / f"synthetic_{index:03d}.xdi"
        header, rows = spectrum(index, args.three_signals)
        with path.open("x", encoding="utf-8") as stream:
            stream.write(header)
            stream.writelines(rows[:150])
            stream.flush()
            time.sleep(args.partial_pause)
            stream.writelines(rows[150:])
        print(path.name, flush=True)
        time.sleep(args.interval)


if __name__ == "__main__":
    main()
