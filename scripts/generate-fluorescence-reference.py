#!/usr/bin/env -S uv run --script
# /// script
# requires-python = "==3.12.*"
# dependencies = ["numpy==2.3.2", "scipy==1.16.1", "xraydb==4.5.8"]
# ///
"""Generate synthetic references with pinned Larch FLUO and conventional preedge.

Only selected numerical function definitions are loaded. The interactive group
adapter is a local shim and the call-argument decorator is removed; numerical
bodies are unchanged. No rexafs output participates. Geometry, energy origin,
intervals, degree and detected line/family are all explicit.
"""
import ast
import hashlib
import json
import platform
from pathlib import Path
from types import SimpleNamespace
from urllib.request import urlopen
import numpy as np
import scipy
import xraydb

COMMIT = 'e3c93284fed358c2c8979cba4c139430527433c6'
BASE = f'https://raw.githubusercontent.com/xraypy/xraylarch/{COMMIT}/larch/'
SOURCES = {}
namespace = dict(np=np, MAX_NNORM=5, TINY_ENERGY=1e-7,
                 xray_line=xraydb.xray_line, xray_edge=xraydb.xray_edge,
                 material_mu=xraydb.material_mu,
                 parse_group_args=lambda energy, members, defaults, group, fcn_name: (energy, defaults[0], group),
                 set_xafsGroup=lambda group, **kw: group)


def load(path, names):
    url = BASE + path
    source = urlopen(url, timeout=30).read()
    SOURCES[path] = dict(url=url, sha256=hashlib.sha256(source).hexdigest(), functions=names)
    tree = ast.parse(source)
    tree.body = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
    assert len(tree.body) == len(names)
    for node in tree.body: node.decorator_list = []
    exec(compile(tree, url, 'exec'), namespace)


def case(formula, element, angles, line, family, degree):
    e0=xraydb.xray_edge(element,'K').energy
    energy=e0-400+np.arange(351)*4.
    energy[1:-1]+=0.1*np.sin(np.arange(1,350))
    energy[100]=e0
    offset=energy-e0
    mu=0.2+1e-5*offset+1/(1+np.exp(-offset/2)) + 0.1*np.exp(-((offset-14)/8)**2)
    pre=energy[[5,75]]-e0
    post=energy[[130,345]]-e0
    settings=dict(e0=e0,pre1=pre[0],pre2=pre[1],norm1=post[0],norm2=post[1],nnorm=degree,nvict=0)
    result=SimpleNamespace()
    namespace['fluo_corr'](energy,mu,formula,element,group=result,edge='K',line=line,
                           anginp=angles[0],angout=angles[1],**settings)
    internal=namespace['preedge'](energy,mu,**settings)
    emission=xraydb.xray_line(element,line).energy
    attenuation=xraydb.material_mu(formula,[emission,e0-10,e0+10],density=1)
    g=np.sin(np.deg2rad(angles[0]))/np.sin(np.deg2rad(angles[1]))
    alpha=(attenuation[0]*g+attenuation[1])/(attenuation[2]-attenuation[1])
    return dict(formula=formula,element=element,edge='K',e0=e0,line=line,family=family,degree=degree,
                angles=angles,pre_edge=pre.tolist(),post_edge=post.tolist(),energy=energy.tolist(),mu=mu.tolist(),
                emission_ev=emission,attenuation=attenuation.tolist(),geometry_ratio=g,alpha=alpha,
                internal_norm=internal['norm'].tolist(),pre_curve=internal['pre_edge'].tolist(),post_curve=internal['post_edge'].tolist(),
                corrected_mu=result.mu_corr.tolist(),final_norm=result.norm_corr.tolist())


if __name__ == '__main__':
    load('math/utils.py',['index_of','index_nearest','remove_dups','remove_nans','remove_nans2','polyfit'])
    load('xafs/pre_edge.py',['preedge'])
    load('xafs/fluo.py',['fluo_corr'])
    cases=[case('CuO','Cu',[45.,45.],'Ka1',False,1),
           case('Cu0.001SiO2','Cu',[30.,60.],'Ka',True,2),
           case('Fe2O3','Fe',[90.,35.],'Ka1',False,1)]
    output=Path(__file__).resolve().parents[1]/'crates/rexafs/tests/fixtures/analysis/fluorescence/larch-reference.json'
    output.parent.mkdir(parents=True,exist_ok=True)
    artifact=dict(provenance=dict(generator='scripts/generate-fluorescence-reference.py',kind='synthetic',larch_commit=COMMIT,sources=SOURCES,
                  python=platform.python_version(),numpy=np.__version__,scipy=scipy.__version__,xraydb=xraydb.__version__,database=xraydb.get_xraydb().get_version(),
                  license=f'https://github.com/xraypy/xraylarch/blob/{COMMIT}/LICENSE'),
                  tolerances=dict(curve_absolute=1e-7,atomic_relative=2e-10),cases=cases)
    output.write_text(json.dumps(artifact,separators=(',',':'),allow_nan=False)+'\n')
    print(output)
