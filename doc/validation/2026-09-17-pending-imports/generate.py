"""Generate synthetic inputs for pending-import GUI checks, without experimental data."""

import math
from pathlib import Path
import sys


def generate(root: Path) -> None:
    """Create a new directory; refuse to overwrite an existing test or data folder."""
    root.mkdir(parents=True, exist_ok=False)
    pending = root / "pending"
    pending.mkdir()
    energy = [8750.0 + 2.0 * i for i in range(401)]
    signal = [
        1 / (1 + math.exp(-(e - 8980.0) / 3.0))
        + 0.12 * math.exp(-((e - 8996.0) / 12.0) ** 2)
        for e in energy
    ]
    rows = "".join(f"{e} {y}\n" for e, y in zip(energy, signal))
    athena = (
        "# Athena project file -- Demeter version 0.9.26\n"
        "$old_group = 'synth';\n"
        "@args = ('label', 'Synthetic QA', 'is_xmu', 1);\n"
        "@x = (" + ",".join(map(str, energy)) + ");\n"
        "@y = (" + ",".join(map(str, signal)) + ");\n[record]\n"
    )
    for name in ["optional-project.prj", "second-project.PRJ", "optional-checkpoint.xts"]:
        (pending / name).write_text(athena)
    spec = "#F synthetic.spec\n#S 1 Synthetic QA\n#N 2\n#L energy  mu\n" + rows
    for name in ["keep-spectrum.spec", "skip-spectrum.spec"]:
        (pending / name).write_text(spec)
    (root / "accepted.xdi").write_text(
        "# XDI/1.0 rexafs/qa\n# Element.symbol: Cu\n# Element.edge: K\n"
        "# Column.1: energy eV\n# Column.2: mu\n# ///\n"
        "# Generated software test signal\n#-----\n# energy mu\n" + rows
    )


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 generate.py NEW_OUTPUT_DIRECTORY")
    generate(Path(sys.argv[1]))
