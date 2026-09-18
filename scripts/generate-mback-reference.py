#!/usr/bin/env -S uv run --script
# /// script
# requires-python = "==3.12.*"
# dependencies = ["numpy==2.3.2", "scipy==1.16.1", "lmfit==1.3.4", "xraydb==4.5.8"]
# ///
"""Independent full-MBACK objective and normalization references; synthetic only.

Loads only named functions from pinned Larch sources. No rexafs implementation
or output participates in the reference. Identical explicit regions, polynomial
space, positive scale and erfc bounds are supplied to Larch's match_f2 objective.
The optimizer configuration is explicit, not Larch's interactive default start.
"""
import ast
import hashlib
import json
import platform
from pathlib import Path
from urllib.request import urlopen
import lmfit
import numpy as np
import scipy
from scipy.special import erfc
import xraydb

COMMIT = 'e3c93284fed358c2c8979cba4c139430527433c6'
BASE = f'https://raw.githubusercontent.com/xraypy/xraylarch/{COMMIT}/larch/'
SOURCES = {}
namespace = dict(np=np, erfc=erfc, MAX_NNORM=5, TINY_ENERGY=1.e-7)


def load(path, names):
    url = BASE + path
    source = urlopen(url, timeout=30).read()
    SOURCES[path] = dict(url=url, sha256=hashlib.sha256(source).hexdigest(), functions=names)
    tree = ast.parse(source)
    tree.body = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
    assert len(tree.body) == len(names)
    exec(compile(tree, url, 'exec'), namespace)


def case(with_erfc):
    energy = 8579 + np.arange(351) * 4.
    energy[1:-1] += 0.15*np.sin(np.arange(1, 350))
    energy[100] = 8979.
    e0 = 8979.
    pre = energy[[5,75]] - e0
    post = energy[[135,345]] - e0
    f2 = xraydb.f2_chantler('Cu',energy)
    emission = xraydb.xray_line('Cu','Ka1').energy
    offset = energy-e0
    degree = 1 if with_erfc else 2
    background = 0.7 + 0.001*offset + (0 if with_erfc else 1e-6*offset**2)
    if with_erfc: background += 3*erfc((energy-emission)/900.)
    mu = (f2+background)/2.3 + 0.0003*np.sin(np.arange(len(energy))*0.37)
    mu[(offset>0)&(offset<50)] += 0.8  # Excluded synthetic white-line structure.
    p = lmfit.Parameters()
    p.add('s',value=2.,min=1e-12)
    p.add('a',value=2. if with_erfc else 0.,vary=with_erfc,min=0,max=10)
    p.add('xi',value=800. if with_erfc else 50.,vary=with_erfc,min=500 if with_erfc else 1,max=1500 if with_erfc else 2000)
    for j in range(degree+1): p.add(f'c{j}',value=0.)
    theta=np.zeros_like(energy)
    weights=np.ones_like(energy)
    for start,end in [pre,post]:
        mask=(offset>=start)&(offset<=end)
        theta[mask]=1
        weights[mask]=np.sqrt(mask.sum())
    kwargs=dict(en=energy,mu=mu,f2=f2,e0=e0,em=emission,order=degree,theta=theta,weight=weights)
    result=lmfit.minimize(namespace['match_f2'],p,method='least_squares',kws=kwargs,
                          x_scale='jac',max_nfev=5000,ftol=1e-13,xtol=1e-13,gtol=1e-13)
    assert result.success, result.message
    values=result.params.valuesdict()
    background=sum(values[f'c{j}']*offset**j for j in range(degree+1))
    background+=values['a']*erfc((energy-emission)/values['xi'])
    aux=namespace['preedge'](energy,f2+background,e0=e0,pre1=pre[0],pre2=pre[1],norm1=post[0],norm2=post[1],nnorm=2,nvict=0)
    return dict(erfc=with_erfc,element='Cu',edge='K',e0=e0,degree=degree,energy=energy.tolist(),mu=mu.tolist(),
                pre_edge=pre.tolist(),post_edge=post.tolist(),scale=values['s'],coefficients=[values[f'c{j}'] for j in range(degree+1)],
                erfc_width=values['xi'],erfc_amplitude=values['a'],emission=emission,
                fit_indices=np.flatnonzero(theta).tolist(),f2=f2.tolist(),background=background.tolist(),fpp=(values['s']*mu-background).tolist(),
                norm=((values['s']*mu-aux['pre_edge'])/aux['edge_step']).tolist(),edge_step=aux['edge_step']/values['s'],
                pre_curve=aux['pre_edge'].tolist(),post_curve=aux['post_edge'].tolist(),objective=float(np.dot(result.residual,result.residual)))


if __name__ == '__main__':
    load('math/utils.py',['index_of','index_nearest','remove_dups','remove_nans','remove_nans2','polyfit'])
    load('xafs/pre_edge.py',['preedge'])
    load('xafs/mback.py',['match_f2'])
    cases=[case(False),case(True)]
    output=Path(__file__).resolve().parents[1]/'crates/rexafs/tests/fixtures/analysis/mback/larch-reference.json'
    output.parent.mkdir(parents=True,exist_ok=True)
    artifact=dict(provenance=dict(generator='scripts/generate-mback-reference.py',kind='synthetic',larch_commit=COMMIT,sources=SOURCES,
                                  python=platform.python_version(),numpy=np.__version__,scipy=scipy.__version__,lmfit=lmfit.__version__,
                                  xraydb=xraydb.__version__,database=xraydb.get_xraydb().get_version(),
                                  license='https://github.com/xraypy/xraylarch/blob/'+COMMIT+'/LICENSE'),
                  tolerances=dict(curve_absolute=1e-6,scale_absolute=1e-7,objective_absolute=1e-10,erfc_width_absolute=0.02),cases=cases)
    output.write_text(json.dumps(artifact,separators=(',',':'),allow_nan=False)+'\n')
    print(output)
