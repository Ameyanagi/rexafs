#!/usr/bin/env python3
"""Generate Larix reader fixtures using the pinned upstream session writer.

Run with the environment in requirements-lock.txt. This overwrites generated
fixtures and their manifests, but never the original source measurements.
"""

import copy
import gzip
import importlib.metadata
import io
import json
import os
import platform
import sys
from contextlib import ExitStack, redirect_stdout, redirect_stderr
from datetime import datetime
from pathlib import Path
from unittest.mock import patch

# lmfit's saved unique-symbol dictionary is built from a set. Fix the process
# hash seed before importing it so the upstream writer emits stable key order.
if __name__ == '__main__' and os.environ.get('PYTHONHASHSEED') != '0':
    os.execve(sys.executable, [sys.executable, *sys.argv], {**os.environ, 'PYTHONHASHSEED': '0'})

import larch
import numpy as np
from larch import Group, Interpreter, Journal
from larch.io import read_ascii, read_session, save_session
from larch.io import save_restore
from larch.utils import jsonutils
from larch.utils.physical_constants import PLANCK_HC
from larch.xafs import autobk, pre_edge, xftf
from lmfit import Parameters

from fixturelib import compare, digest, snapshot

ROOT = Path(__file__).resolve().parent
STAMP = datetime(2026, 9, 14, 0, 0, 0)
PRE = dict(pre1=-150, pre2=-30, norm1=100, norm2=800, nnorm=2)
BG = dict(rbkg=1, kmin=0, kmax=12, kweight=1, dk=1, nfft=2048, kstep=0.05)
FT = dict(kmin=2, kmax=10, kweight=2, dk=1, window='hanning', nfft=2048, kstep=0.05)


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False, allow_nan=False) + '\n')


def raw_group(filename, name, mode):
    source = ROOT / 'sources' / filename
    group = read_ascii(str(source), labels='angle_c angle_o dwell i0 signal')
    # Preserve every source row. Offset corrections are not applied a second time.
    group.path = f'sources/{filename}'
    group.filename = filename
    group.groupname = name
    group.datatype = 'xas'
    group.energy_units = 'eV'
    group.energy = PLANCK_HC / (2 * 3.13551 * np.sin(np.deg2rad(group.angle_o)))
    group.mu = np.log(group.i0 / group.signal) if mode == 'transmission' else group.signal / group.i0
    if mode == 'transmission':
        group.itrans = group.signal.copy()
    else:
        group.ifluor = group.signal.copy()
    group.xdat = group.energy.copy()
    group.ydat = group.mu.copy()
    group.xplot = group.energy.copy()
    group.yplot = group.mu.copy()
    group.source_info = dict(filename=filename, sha256=digest(source), mode=mode,
                             angle_column='Angle(o)', d_spacing_angstrom=3.13551,
                             planck_hc_ev_angstrom=PLANCK_HC)
    group.journal = Journal()
    group.journal.add('source', group.source_info, dtime=STAMP)
    return group


def checkpoint(raw, stage):
    group = copy.deepcopy(raw)
    # A derived checkpoint keeps the first measurement at each duplicate energy.
    _, indices = np.unique(group.energy, return_index=True)
    indices.sort()
    count = len(group.energy)
    for key in dir(group):
        val = getattr(group, key)
        if isinstance(val, np.ndarray) and val.shape == (count,):
            setattr(group, key, val[indices].copy())
    group.data = group.data[:, indices].copy()
    group.retained_source_rows = indices.astype(np.int64)
    group.source_info['removed_duplicate_rows'] = count - len(indices)
    group.journal.add('duplicate_policy', 'Keep first row at each observed energy; preserve order.',
                      dtime=STAMP)
    pre_edge(group, **PRE)
    if stage in ('autobk', 'ft'):
        autobk(group, **BG)
    if stage == 'ft':
        xftf(group, **FT)
    # Larch's processing decorators append wall-clock Journal entries.
    group.journal = Journal([(e.key, e.value, STAMP) for e in group.journal])
    group.fixture_stage = stage
    return group


def deterministic_gzip(filename, mode):
    return gzip.GzipFile(filename, mode, mtime=0)


def save_case(name, groups, description, *, legacy=False, auto=True, history=None, extras=None):
    symbols = {g.groupname: g for g in groups}
    symbols.update(extras or {})
    session = Interpreter()
    # A controlled fixture session does not include the user's preferences or paths.
    session.symtable._sys.config = Group(fixture_corpus='larix-data', fixture_schema=1)
    for key, value in symbols.items():
        setattr(session.symtable, key, value)
    history = history if history is not None else ['# Fixture created by generate.py using Larch APIs.']
    expected_symbols = dict(symbols)
    if auto:
        expected_symbols = {'_xasgroups': {g.filename: g.groupname for g in groups}, **symbols}
    expected = dict(symbols=snapshot(expected_symbols), command_history=history)
    path = ROOT / 'fixtures' / 'valid' / f'{name}.larix'
    with ExitStack() as stack:
        stack.enter_context(patch.object(jsonutils, 'USE_NPJSON', not legacy))
        stack.enter_context(patch.object(save_restore.socket, 'gethostname', return_value='fixture-host'))
        stack.enter_context(patch.object(save_restore, 'get_machineid', return_value='000000000000'))
        stack.enter_context(patch.object(save_restore.time, 'strftime', return_value=STAMP.isoformat(' ')))
        stack.enter_context(patch.object(save_restore, 'GzipFile', deterministic_gzip))
        save_session(str(path), symbols=list(symbols), histbuff=history,
                     auto_xasgroups=auto, _larch=session)
    noise = io.StringIO()
    with redirect_stdout(noise), redirect_stderr(noise):
        restored = read_session(str(path), clean_xasgroups=False)
    if noise.getvalue():
        raise RuntimeError(f'{name}: Larch reported a decode problem: {noise.getvalue()}')
    compare(snapshot(restored.symbols), expected['symbols'], name)
    assert restored.command_history == history, f'{name}: history round trip differs'
    assert restored.config['Larix Version'] == '1.0'
    records.append(dict(path=path.relative_to(ROOT).as_posix(), category='valid',
                        description=description, generation='larch.io.save_session',
                        array_encoding='Array' if legacy else 'b64ndarray',
                        expected_absorption_groups=[g.groupname for g in groups
                                                    if hasattr(g, 'energy') and hasattr(g, 'mu')],
                        larch_read_session_verified=True))
    expectations[path.relative_to(ROOT).as_posix()] = expected
    print(f'Verified {path.name}', flush=True)
    return path


def mutate_symbol(text, symbol, transform):
    lines = text.splitlines()
    index = lines.index(f'<:{symbol}:>') + 1
    obj = json.loads(lines[index])
    transform(obj)
    lines[index] = json.dumps(obj)
    return '\n'.join(lines) + '\n'


def main():
    if larch.__version__ != '2026.3.1':
        raise RuntimeError('Generation requires xraylarch==2026.3.1')
    for directory in ('valid', 'invalid'):
        (ROOT / 'fixtures' / directory).mkdir(parents=True, exist_ok=True)
    for source in json.loads((ROOT / 'sources/manifest.json').read_text()):
        assert digest(ROOT / source['path']) == source['sha256'], source['path']

    transmission = raw_group('PFBL12C_2005.dat', 'pfbl12c', 'transmission')
    fluorescence = raw_group('PF9A_2022.dat', 'pf9a', 'fluorescence')
    assert len(transmission.energy) == 818 and len(fluorescence.energy) == 1426
    save_case('pfbl12c-raw', [transmission], 'Transmission; 818 original energy/mu and detector rows.')
    save_case('pf9a-raw', [fluorescence], 'Fluorescence; 1426 rows, including six duplicate energies.')
    analyzed = {}
    for stage in ('normalized', 'autobk', 'ft'):
        group = checkpoint(transmission, stage)
        save_case(f'pfbl12c-{stage}', [group], f'Transmission after {stage}; 818 absorption points.')
        analyzed[stage] = group
    fluor_ft = checkpoint(fluorescence, 'ft')
    assert len(fluor_ft.energy) == 1420
    save_case('pf9a-ft', [fluor_ft], 'Fluorescence after normalization, background and FT; 1420 unique energies.')
    save_case('two-analyzed', [analyzed['ft'], fluor_ft], 'Two processed groups on independent energy grids.')
    save_case('mixed-raw-analyzed', [transmission, fluor_ft], 'Raw transmission and processed fluorescence.')
    save_case('legacy-arrays', [analyzed['ft']], 'Legacy numeric JSON arrays, including complex FT values.', legacy=True)
    save_case('no-xasgroups', [transmission], 'Valid session saved without the optional _xasgroups index.', auto=False)

    typed = Group(groupname='typed', filename='Fe foil — 日本語 μ(E).dat', datatype='xas',
                  energy_units='eV', energy=np.array([7100., 7110., 7120., 7130.]),
                  mu=np.array([0.1, 0.2, 1.2, 1.1]))
    typed.xdat = typed.energy.copy()
    typed.ydat = typed.mu.copy()
    typed.journal = Journal()
    typed.journal.add('comment', 'first entry', dtime=STAMP)
    typed.journal.add('comment', 'second entry; preserve repeated keys', dtime=STAMP)
    typed.metadata = dict(unicode='日本語, μ(E), Å', empty=None, flag=True,
                          labels=('sample', 'reference'), timestamp=STAMP.isoformat(),
                          relative_path='sources/fictional scan.dat',
                          matrix=np.arange(6, dtype=np.float32).reshape(2, 3),
                          big_endian=np.array([1.5, 2.5], dtype='>f8'),
                          counts=np.array([0, 2**53 + 1], dtype=np.int64),
                          mask=np.array([True, False]), empty_array=np.empty((0, 3)),
                          complex_scalar=1+2j)
    typed.params = Parameters()
    typed.params.add('amplitude', value=0.9, min=0, max=2)
    typed.params.add('twice_amplitude', expr='2 * amplitude')
    typed_file = save_case('typed-metadata', [typed], 'Synthetic names, metadata, Journals, typed arrays and parameter expressions.',
                          history=['# Synthetic command history; retained as text.', 'fixture_marker = "history text only"'],
                          extras={'session_note': 'Not a spectrum', 'ordinal': 7})
    chi = Group(groupname='chi_only', filename='chi-only.dat', datatype='chi',
                k=analyzed['ft'].k.copy(), chi=analyzed['ft'].chi.copy())
    save_case('chi-only', [chi], 'EXAFS chi(k) group with no absorption arrays; no automatic energy/mu mapping.')
    xy = Group(groupname='lineup', filename='line-up.dat', datatype='xydata',
               xdat=np.array([-1., 0., 1.]), ydat=np.array([1., 3., 1.]), xunits='mm')
    save_case('non-xas', [xy], 'Synthetic position scan; xdat is millimeters, not photon energy.')
    save_case('empty', [], 'Valid empty session with an empty _xasgroups dictionary.', history=[])

    # Plain text is a reader compatibility variant; save_session itself writes gzip.
    plain = ROOT / 'fixtures/valid/plain-session.larix'
    plain.write_bytes(gzip.decompress(typed_file.read_bytes()))
    expected = expectations[typed_file.relative_to(ROOT).as_posix()]
    restored = read_session(str(plain), clean_xasgroups=False)
    assert snapshot(restored.symbols) == expected['symbols']
    assert restored.command_history == expected['command_history']
    expectations[plain.relative_to(ROOT).as_posix()] = copy.deepcopy(expected)
    records.append(dict(path=plain.relative_to(ROOT).as_posix(), category='valid',
                        description='Exact decompressed typed-metadata session.',
                        generation='gzip.decompress(typed-metadata.larix)',
                        array_encoding='b64ndarray', expected_absorption_groups=['typed'],
                        larch_read_session_verified=True))

    text = plain.read_text()
    broken_json = text.replace('<:typed:>\n{', '<:typed:>\n!', 1)
    def bad_shape(obj):
        inner = json.loads(obj['energy']['__value__'])
        inner['shape'] = [5]
        obj['energy']['__value__'] = json.dumps(inner)
    def short_mu(obj):
        obj['mu'] = jsonutils.encode4js(np.array([0.1, 0.2, 1.2]))
    negatives = [
        ('truncated-gzip', typed_file.read_bytes()[:-12], 'gzip', 'Truncated gzip stream; reject damaged input.'),
        ('bad-magic', gzip.compress(text.replace('##LARIX:', '##OTHER:', 1).encode(), mtime=0),
         'format', 'Header does not identify a Larix session.'),
        ('invalid-json', gzip.compress(broken_json.encode(), mtime=0), 'json', 'Invalid JSON for the typed symbol; reject incomplete decoding.'),
        ('bad-array-shape', gzip.compress(mutate_symbol(text, 'typed', bad_shape).encode(), mtime=0),
         'array', 'Array shape requires five float64 values but only four are stored.'),
        ('mismatched-energy-mu', gzip.compress(mutate_symbol(text, 'typed', short_mu).encode(), mtime=0),
         'mapping', 'Serialization is valid; four energy points and three mu values cannot map to a spectrum.'),
        ('missing-group-reference', gzip.compress(mutate_symbol(text, '_xasgroups',
            lambda obj: obj.update({'missing.dat': 'missing_group'})).encode(), mtime=0),
         'reference', 'Serialization is valid; report a dangling _xasgroups reference without hiding it.'),
    ]
    for name, payload, layer, description in negatives:
        path = ROOT / 'fixtures/invalid' / f'{name}.larix'
        path.write_bytes(payload)
        records.append(dict(path=path.relative_to(ROOT).as_posix(), category='invalid',
                            generation='Documented mutation of typed-metadata.larix; see generate.py.',
                            expected_failure_layer=layer, description=description))

    write_json(ROOT / 'expected.json', dict(schema_version=1,
               array_hash='SHA-256 of all values in original dtype normalized to little endian, C order',
               sessions=expectations))
    source_files = ['larch/io/save_restore.py', 'larch/utils/jsonutils.py', 'larch/utils/npjson.py']
    package_root = Path(larch.__file__).parent.parent
    manifest = dict(schema_version=1, format='Larix session 1.0',
        generation=dict(method='Larch Python API; no GUI interaction', larch=larch.__version__,
                        python=platform.python_version(), platform=platform.system(),
                        dependencies={p: importlib.metadata.version(p) for p in ('numpy', 'scipy', 'lmfit', 'xraydb')},
                        writer_sources=[dict(path=p, sha256=digest(package_root / p),
                            url=f'https://github.com/xraypy/xraylarch/blob/2026.3.1/{p}') for p in source_files],
                        controlled_metadata='Fixed fixture timestamp, hostname and MAC ID; isolated config; gzip mtime=0; PYTHONHASHSEED=0.',
                        pre_edge=PRE, autobk=BG, xftf=FT),
        files=records,
        assets=[])
    for record in records:
        path = ROOT / record['path']
        record.update(bytes=path.stat().st_size, sha256=digest(path))
    for path in sorted(ROOT.rglob('*')):
        rel = path.relative_to(ROOT)
        if (path.is_file() and rel.parts[0] not in ('.git', '.venv', '__pycache__', 'fixtures')
                and path.name not in ('manifest.json', '.DS_Store')):
            manifest['assets'].append(dict(path=rel.as_posix(), bytes=path.stat().st_size, sha256=digest(path)))
    # The source manifest is included explicitly because the bundle manifest is excluded above.
    source_manifest = ROOT / 'sources/manifest.json'
    manifest['assets'].append(dict(path='sources/manifest.json', bytes=source_manifest.stat().st_size,
                                   sha256=digest(source_manifest)))
    write_json(ROOT / 'manifest.json', manifest)
    print(f'Generated {len(records)} fixtures with pre-serialization expectations.')


records = []
expectations = {}
if __name__ == '__main__':
    main()
