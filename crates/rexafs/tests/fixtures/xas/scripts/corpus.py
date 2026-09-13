#!/usr/bin/env python3
"""List, restore and verify the collected corpus; does not implement XAS parsing."""

import argparse
import gzip
import hashlib
import json
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def local_path(relative):
    path = (ROOT / relative).resolve()
    if not path.is_relative_to(ROOT):
        raise ValueError(f"Path outside corpus: {relative}")
    return path


def selected(manifest, include_candidates):
    return [r for r in manifest['samples'] if include_candidates or
            r['test_eligibility'] == 'documented-license']


def inspect_payload(record, data, hdf5=False):
    """Container/readability checks only; numeric rows are not parsed scan points."""
    out = {'id': record['id'], 'path': record['path'], 'sha256': digest(data)}
    if data.startswith(b'\x89HDF\r\n\x1a\n'):
        out['container'] = 'HDF5'
        if not hdf5:
            out['readability'] = 'magic verified; use --hdf5 to read datasets'
            return out
        import h5py
        out['datasets'] = []
        with h5py.File(local_path(record['path']), 'r') as handle:
            def visit(name, obj):
                if isinstance(obj, h5py.Dataset):
                    # The collected HDF5 payloads are small enough to read fully.
                    obj[()]
                    out['datasets'].append({'path': name, 'shape': list(obj.shape),
                                            'dtype': str(obj.dtype)})
            handle.visititems(visit)
        if not out['datasets']:
            raise ValueError('HDF5 contains no readable datasets')
        out['readability'] = 'all visited dataset payloads read with h5py'
        return out
    if data.startswith(b'\x1f\x8b'):
        data = gzip.decompress(data)
        out['compression'] = 'gzip (CRC checked on decompression)'
    if data.lstrip().startswith((b'<!DOCTYPE html', b'<html', b'<HTML')):
        raise ValueError('HTML response instead of measurement')
    if data.startswith(b'version https://git-lfs.github.com/spec'):
        raise ValueError('Git LFS pointer instead of measurement')
    if 'binary' in record['format'].lower():
        out['container'] = record['format']
        out['readability'] = 'byte integrity only; legacy binary parser not run'
        return out
    encoding = 'utf-8'
    try:
        text = data.decode(encoding)
    except UnicodeDecodeError:
        encoding = 'latin-1'
        text = data.decode(encoding)
    out['inspection_encoding'] = encoding
    if text.lstrip().startswith('{'):
        project = json.loads(text)
        out['container'] = 'JSON'
        out['top_level_entries'] = len(project)
    elif 'Athena project file' in text[:300]:
        out['container'] = 'Athena Perl serialization'
        out['serialized_x_arrays'] = len(re.findall(r'^@x\s*=', text, re.M))
        if not out['serialized_x_arrays']:
            raise ValueError('Athena project has no serialized x arrays')
    else:
        widths = {}
        for line in text.splitlines():
            if line.lstrip().startswith(('#', '!', ';')):
                continue
            fields = re.split(r'[\s,]+', line.strip().strip(','))
            if len(fields) < 2:
                continue
            try:
                [float(f.replace('D', 'E').replace('d', 'e')) for f in fields]
            except ValueError:
                continue
            key = str(len(fields))
            widths[key] = widths.get(key, 0) + 1
        out['container'] = 'text'
        out['numeric_line_widths'] = widths
        out['readability'] = 'numeric text found; includes header/continuation rows'
        if sum(widths.values()) < 10:
            # LNLS includes date/time columns alongside numbers.
            if 'date/time' not in record['format']:
                raise ValueError('Too few numeric text lines')
            out['readability'] = 'date/time table retained; no generic numeric row count'
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['verify', 'list', 'fetch'])
    parser.add_argument('--include-candidates', action='store_true',
                        help='Include locally collected RefXAS review-required entries')
    parser.add_argument('--hdf5', action='store_true', help='Read HDF5 using optional h5py')
    parser.add_argument('--report', type=Path, help='Write verification observations as JSON')
    args = parser.parse_args()
    manifest = json.loads((ROOT / 'manifest.json').read_text())
    records = selected(manifest, args.include_candidates)
    if args.command == 'list':
        print('\n'.join(r['path'] for r in records))
        return
    if args.command == 'fetch':
        for r in records:
            path = local_path(r['path'])
            if path.exists():
                if digest(path.read_bytes()) != r['sha256']:
                    raise ValueError(f"Refusing to overwrite changed file: {path}")
                continue
            data = urllib.request.urlopen(r['download_url'], timeout=60).read()
            if len(data) != r['bytes'] or digest(data) != r['sha256']:
                raise ValueError(f"Source content changed: {r['id']}")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            if r.get('sidecar_text'):
                Path(str(path) + '.license').write_text(r['sidecar_text'])
            print(f"Restored {r['path']}")
        return
    ids, paths, observations = set(), set(), []
    for r in records:
        if r['id'] in ids or r['path'] in paths:
            raise ValueError(f"Duplicate ID/path: {r['id']}")
        ids.add(r['id'])
        paths.add(r['path'])
        for key in ['citation', 'license', 'license_scope', 'license_evidence_url',
                    'source_url', 'download_url', 'beamline_evidence']:
            if not r.get(key):
                raise ValueError(f"Missing {key}: {r['id']}")
        for key in ['license_file', 'license_evidence_file']:
            if r.get(key) and not local_path(r[key]).is_file():
                raise ValueError(f"Missing {key}: {r['id']}")
        for companion in r.get('companion_files', []):
            if not local_path(companion).is_file():
                raise ValueError(f"Missing companion: {companion}")
        data = local_path(r['path']).read_bytes()
        if len(data) != r['bytes'] or digest(data) != r['sha256']:
            raise ValueError(f"Integrity mismatch: {r['path']}")
        if r.get('upstream_git_blob_sha1'):
            blob = b'blob ' + str(len(data)).encode() + b'\0' + data
            if hashlib.sha1(blob).hexdigest() != r['upstream_git_blob_sha1']:
                raise ValueError(f"Git blob mismatch: {r['path']}")
        sidecar = local_path(r['path'] + '.license')
        if sidecar.read_text() != r['sidecar_text']:
            raise ValueError(f"Attribution sidecar mismatch: {sidecar}")
        observations.append(inspect_payload(r, data, args.hdf5))
    for asset in manifest['assets']:
        data = local_path(asset['path']).read_bytes()
        if len(data) != asset['bytes'] or digest(data) != asset['sha256']:
            raise ValueError(f"Asset integrity mismatch: {asset['path']}")
    report = {'scope': 'Integrity and container checks, not rexafs parser tests',
              'files_verified': len(records), 'assets_verified': len(manifest['assets']),
              'hdf5_read': args.hdf5, 'observations': observations}
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(f"Verified {len(records)} measurement files and {len(manifest['assets'])} assets.")


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, ImportError) as exc:
        sys.exit(str(exc))
