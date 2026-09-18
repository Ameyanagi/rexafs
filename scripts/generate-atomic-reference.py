#!/usr/bin/env -S uv run --script
# /// script
# requires-python = "==3.12.*"
# dependencies = ["numpy==2.3.2", "scipy==1.16.1", "xraydb==4.5.8"]
# ///
"""Record offline atomic-table reference queries, without experimental inputs.

The returned values are reference data, not experimentally measured spectra.
XrayDB's database compilation is CC0; retain its full underlying-source caveats
in crates/rexafs/data/atomic/XrayDB-LICENSE.txt. Python is only a fixture generator.
"""
import hashlib
import json
import platform
from pathlib import Path
import numpy as np
import scipy
import xraydb


def generate():
    db = xraydb.get_xraydb()
    cases = []
    for element, shell in [('Cu', 'K'), ('Fe', 'K'), ('Ge', 'K'), ('Au', 'L3')]:
        edge = xraydb.xray_edge(element, shell).energy
        energy = edge + np.array([-300., -10., -0.1, 0., 0.1, 10., 123.4, 800.])
        lines = xraydb.xray_lines(element, shell)
        cases.append(dict(element=element, shell=shell, edge=edge, energy=energy.tolist(),
                          f2=np.asarray(xraydb.f2_chantler(element, energy)).tolist(),
                          attenuation=np.asarray(xraydb.mu_elam(element, energy, kind='total')).tolist(),
                          lines={key: dict(energy=v.energy, intensity=v.intensity) for key,v in sorted(lines.items())}))
    energy = np.array([7000., 8046.3, 8969., 8989., 11000.])
    compounds = []
    for formula in ['Cu2O', 'H2O', 'CuSO4(H2O)5', 'Fe0.7Mg0.3O']:
        # Density 1 g/cm^3 makes material_mu numerically equal to mass attenuation.
        # rexafs does not take a density when returning a mass coefficient.
        values = xraydb.material_mu(formula, energy, density=1., kind='total')
        compounds.append(dict(formula=formula, energy=energy.tolist(), values=values.tolist()))
    provenance = dict(generator='scripts/generate-atomic-reference.py', kind='atomic reference queries',
                      python=platform.python_version(), xraydb=xraydb.__version__, database=db.get_version(),
                      database_sha256=hashlib.sha256(Path(db.dbname).read_bytes()).hexdigest(),
                      numpy=np.__version__, scipy=scipy.__version__,
                      source='https://xraypy.github.io/XrayDB/python.html',
                      license='../../../../data/atomic/XrayDB-LICENSE.txt')
    return dict(provenance=provenance, tolerance_relative=2e-10, cases=cases, compounds=compounds)


if __name__ == '__main__':
    path = Path(__file__).resolve().parents[1] / 'crates/rexafs/tests/fixtures/analysis/atomic/xraydb-reference.json'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(generate(), indent=2, allow_nan=False) + '\n')
    print(path)
