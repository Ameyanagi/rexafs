#!/usr/bin/env -S uv run --script
# /// script
# requires-python = "==3.12.*"
# dependencies = ["numpy==2.3.2"]
# ///
"""Matched-grid Cauchy references from pinned Larch; synthetic data only.

The numerical function is unchanged. Its decorator and interactive group adapter
are replaced locally. Larch couples order to the number of R rows and uses a
2*nfft transform; the fixture records that order, exact R coordinates and actual
FFT length so rexafs can compare conventions explicitly, not default for default.
"""
import ast
import hashlib
import json
import platform
from pathlib import Path
from types import SimpleNamespace
from urllib.request import urlopen
import numpy as np

COMMIT='e3c93284fed358c2c8979cba4c139430527433c6'
URL=f'https://raw.githubusercontent.com/xraypy/xraylarch/{COMMIT}/larch/xafs/cauchy_wavelet.py'
source=urlopen(URL,timeout=30).read()
tree=ast.parse(source)
tree.body=[node for node in tree.body if isinstance(node,ast.FunctionDef) and node.name=='cauchy_wavelet']
assert len(tree.body)==1
tree.body[0].decorator_list=[]
namespace=dict(np=np,parse_group_args=lambda k,members,defaults,group,fcn_name:(k,defaults[0],group),set_xafsGroup=lambda group,**kw:group)
exec(compile(tree,URL,'exec'),namespace)


def case(step,rmax,weight):
    k=np.arange(81)*step
    chi=np.sin(2*k*0.7)*np.exp(-0.5*((k-2)/0.8)**2)+0.3*np.sin(2*k*1.6)*np.exp(-0.5*((k-3)/0.6)**2)
    output=SimpleNamespace()
    namespace['cauchy_wavelet'](k,chi,output,kweight=weight,rmax_out=rmax,nfft=256)
    return dict(k=k.tolist(),chi=chi.tolist(),kweight=weight,kstep=step,order=len(output.wcauchy_r),
                larch_nfft=256,actual_fft_length=512,r=output.wcauchy_r.tolist(),
                real=output.wcauchy_re.ravel().tolist(),imaginary=output.wcauchy_im.ravel().tolist())


if __name__=='__main__':
    fixture=dict(provenance=dict(generator='scripts/generate-wavelet-reference.py',kind='synthetic',larch_commit=COMMIT,
                 source=dict(url=URL,sha256=hashlib.sha256(source).hexdigest()),python=platform.python_version(),numpy=np.__version__,
                 license=f'https://github.com/xraypy/xraylarch/blob/{COMMIT}/LICENSE',
                 historical_attribution='Larch source retains (c) 2000 Univ. Marne la Vallee, France; Hans-Argoul, Argoul-Munoz and Farges-Munoz; Python translation M Newville (2014).',
                 scientific_reference='https://doi.org/10.2138/am-2003-0423'),
                 absolute_tolerance=2e-11,cases=[case(0.05,1.5,0),case(0.1,2.,2)])
    root=Path(__file__).resolve().parents[1]/'crates/rexafs/tests/fixtures/analysis/wavelet'
    root.mkdir(parents=True,exist_ok=True)
    path=root/'larch-reference.json'
    path.write_text(json.dumps(fixture,separators=(',',':'),allow_nan=False)+'\n')
    (root/'LARCH-LICENSE.txt').write_bytes(urlopen(f'https://raw.githubusercontent.com/xraypy/xraylarch/{COMMIT}/LICENSE',timeout=30).read())
    print(path)
