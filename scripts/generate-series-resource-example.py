#!/usr/bin/env python3
"""Create an entirely synthetic linked-file Series workload outside the checkout.

Each of N distinct paths represents a separate synthetic frame. Repeated signals
share filesystem hard links when available to reduce disk use; their path/frame
identities remain distinct. There are 401 energy points per spectrum and a known
transient at zero-based frame N//2. No experimental or unpublished data is used.
"""
import argparse
import json
import math
import os
import shutil
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--frames', type=int, default=100_000)
    args = parser.parse_args()
    if args.frames < 2:
        parser.error('--frames must be at least 2')
    root = args.directory.resolve()
    frames, bases = root / 'frames', root / 'prototypes'
    if frames.exists() or bases.exists():
        parser.error('Choose a fresh directory; existing frames are never replaced')
    frames.mkdir(parents=True)
    bases.mkdir()
    paths = []
    for index in range(min(args.frames, 1000)):
        amplitude = .12 + .04 * math.sin(index / 40)
        path = bases / f'prototype_{index:04d}.dat'
        path.write_text(spectrum(amplitude), encoding='utf-8')
        paths.append(path)
    for index in range(args.frames):
        dest = frames / f'frame_{index + 1:06d}.dat'
        if index == args.frames // 2:
            dest.write_text(spectrum(1.02), encoding='utf-8')
        else:
            try:
                os.link(paths[index % len(paths)], dest)
            except OSError:
                shutil.copyfile(paths[index % len(paths)], dest)
    project = {
        'version': 1,
        'header': {
            'format': 'rxs', 'format_version': 1,
            'software': 'rexafs synthetic resource generator',
            'software_version': 'development',
            'created_utc': '2026-09-16T00:00:00Z',
            'saved_utc': '2026-09-16T00:00:00Z',
            'storage': 'paths', 'path_base': 'project_directory', 'files': [],
        },
        'source_dir': str(frames),
        'params': {
            'e0': 10000., 'pre_edge_start': -180., 'pre_edge_end': -35.,
            'norm_start': 150., 'norm_end': 990., 'norm_polyorder': 1,
        },
    }
    output = root / 'Synthetic linked series.rxs'
    output.write_text(json.dumps(project), encoding='utf-8')
    print(f'{args.frames} paths; 401 points/frame; transient at frame {args.frames // 2 + 1}; {output}')


def spectrum(amplitude):
    rows = ['# energy mu']
    for i in range(401):
        e = 9800. + 3 * i
        y = (.15 + .00002 * (e - 10000.)
             + 1 / (1 + math.exp(-(e - 10000.) / 2))
             + amplitude * math.exp(-((e - 10025.) / 9) ** 2))
        rows.append(f'{e:.1f} {y:.17g}')
    return '\n'.join(rows) + '\n'


if __name__ == '__main__':
    main()
