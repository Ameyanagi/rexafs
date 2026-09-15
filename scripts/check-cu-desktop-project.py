#!/usr/bin/env python3
"""Verify the retained flat copper GUI experiment and create scientific figures.

Reads a saved project and its coefficient series; does not modify source data.
The expected fractions come from the retained synthetic fixture truth. NumPy
and pyMCR are development references only, not desktop runtime dependencies.
"""
import argparse
import json
from pathlib import Path
from importlib.metadata import version
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from pymcr.mcr import McrAR
import importlib.util
_spec = importlib.util.spec_from_file_location('cu_reference',Path(__file__).with_name('check-cu-mixtures-reference.py'))
_ref = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_ref)
matrix, SimplexRegression = _ref.matrix, _ref.SimplexRegression


def analyze(project, fixtures, output):
    saved=json.loads(project.read_text())
    m=saved['mcr_analysis']; r=m['result']; p=saved['pca_analysis']['model']
    series=saved['lcf_series_analysis']
    assert r['config']['space']==p['space']==series['config']['space']=='Flat'
    assert len(saved['source_groups'])==103 and len(saved['derived'])==8
    assert series['complete'] and not series['cancelled'] and not series['errors']
    assert len(series['rows'])==100
    truth_records=json.loads((fixtures/'truth.json').read_text())
    truth={Path(v['path']).name:np.array(v['weights']) for v in truth_records}
    canonical=['cufoil_abs.xdi','cu2o_abs.xdi','cuo_abs.xdi']
    labels=['Cu foil','Cu₂O','CuO']
    comparison=m['comparison']
    order=[next(i for i,v in enumerate(comparison['inputs']) if v['label']==name) for name in canonical]
    standards=matrix(comparison['spectra'])[order]
    permutation=np.array(comparison['component_indices'])[order]
    data=matrix(r['data']); recovered=matrix(r['spectra'])[permutation]
    coefficients=matrix(r['concentrations'])[:,permutation]
    expected=np.array([truth[v['label']] for v in m['inputs']])
    np.testing.assert_allclose(data,expected@standards,atol=2e-12,rtol=0)
    lcf_order=[next(i for i,v in enumerate(series['standards']) if v['label']==name) for name in canonical]
    lcf_expected=np.array([truth[series['inputs'][str(i)]['label']] for i in range(100)])
    lcf_rows=np.array([series['rows'][str(i)] for i in range(100)])
    lcf_weights=lcf_rows[:,lcf_order]
    np.testing.assert_allclose(lcf_weights,lcf_expected,atol=1e-8,rtol=0)
    centered=matrix(p['data'])-matrix(p['data']).mean(axis=0)
    values=np.linalg.svd(centered,compute_uv=False)**2
    fractions=values/values.sum()
    np.testing.assert_allclose(fractions,p['variance_explained'],atol=1e-12,rtol=0)
    assert sum(fractions[2:])<1e-20
    oracle=McrAR(c_regr=SimplexRegression(),st_regr='OLS',c_constraints=[],st_constraints=[],
                 max_iter=2000,tol_increase=1e-6,tol_n_increase=100,tol_n_above_min=100,tol_err_change=1e-26)
    oracle.fit(data,ST=data[r['initial_samples']].copy())
    difference=float(np.max(np.abs(matrix(r['spectra'])-oracle.ST_opt_)))
    assert difference<1e-8
    spectral_error=np.linalg.norm(recovered-standards,axis=1)/np.linalg.norm(standards,axis=1)
    summary=dict(space='Flat',spectra=100,standards=3,calculated_groups=8,points=data.shape[1],
        versions={name:version(name) for name in ['numpy','pymcr','matplotlib']},
        lcf=dict(max_absolute_weight_error=float(np.max(np.abs(lcf_weights-lcf_expected))),max_r_factor=float(lcf_rows[:,-1].max())),
        pca=dict(centered=True,first_fractions=fractions[:4].tolist(),varying_directions=2),
        mcr=dict(iterations=r['iterations'],relative_squared_residual=r['relative_error'],termination=r['termination'],
            matched_component_indices=permutation.tolist(),relative_spectral_errors=spectral_error.tolist(),
            max_absolute_fraction_error=float(np.max(np.abs(coefficients-expected))),native_pymcr_max_spectral_difference=difference))
    output.mkdir(parents=True,exist_ok=True)
    (output/'flat-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    plt.rcParams.update({'font.size':10})
    colors=['#2166ac','#d95f02','#1b9e77']
    fig,axes=plt.subplots(1,3,figsize=(13,3.8),layout='constrained')
    energy=matrix(r['x']).ravel()
    for j,ax in enumerate(axes):
        ax.plot(energy,standards[j],color=colors[j],lw=2,label='Generating reference')
        ax.plot(energy,recovered[j],color='#333333',ls='--',lw=1.5,label='Blind MCR-ALS')
        ax.set(title=f'{labels[j]} · relative spectral error {spectral_error[j]:.2%}',xlabel='Energy (eV)',ylabel='Flattened μ(E)')
        ax.grid(alpha=.2);ax.legend(fontsize=8)
    fig.savefig(output/'flat-mcr-references.png',dpi=180)
    fig,axes=plt.subplots(1,3,figsize=(13,3.8),layout='constrained')
    for j,label in enumerate(labels):
        axes[0].scatter(lcf_expected[:,j],lcf_weights[:,j],s=13,color=colors[j],label=label)
        axes[2].scatter(expected[:,j],coefficients[:,j],s=13,color=colors[j],label=label)
    for ax in (axes[0],axes[2]):
        ax.plot([0,1],[0,1],color='gray',ls='--',lw=1);ax.set(xlabel='Generating fraction',ylabel='Estimated coefficient');ax.legend(fontsize=8);ax.grid(alpha=.2)
    axes[0].set_title('LCF · all 100 mixtures')
    axes[2].set_title('Blind MCR · reconstruction is not uniqueness')
    axes[1].semilogy(range(1,7),np.maximum(fractions[:6],1e-32),'o-',color='#2166ac')
    axes[1].set(title='Centered PCA · two varying directions',xlabel='Principal component',ylabel='Explained variance fraction');axes[1].grid(alpha=.2)
    fig.savefig(output/'flat-recovery-diagnostics.png',dpi=180)
    print(json.dumps(summary,indent=2))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project',type=Path);parser.add_argument('fixtures',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();analyze(args.project,args.fixtures,args.output)
