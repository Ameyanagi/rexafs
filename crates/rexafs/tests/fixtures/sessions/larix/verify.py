#!/usr/bin/env python3
"""Check fixture integrity offline; optionally reopen all valid files with Larch."""

import argparse
import base64
import gzip
import io
import json
import math
import re
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

from fixturelib import digest


class InvalidFixture(ValueError):
    def __init__(self, layer):
        self.layer = layer
        super().__init__(layer)


def array_shape(value):
    if value.get('__class__') == 'b64ndarray':
        return json.loads(value['__value__'])['shape']
    if value.get('__class__') == 'Array':
        return value['__shape__']
    return None


def check_arrays(value):
    """Inspect declared lengths independently; do not instantiate saved objects."""
    if isinstance(value, list):
        return sum(check_arrays(v) for v in value)
    if not isinstance(value, dict):
        return 0
    tag = value.get('__class__')
    if tag in ('b64ndarray', 'Array'):
        shape = array_shape(value)
        if not all(isinstance(n, int) and n >= 0 for n in shape):
            raise InvalidFixture('array')
        count = math.prod(shape)
        if tag == 'b64ndarray':
            inner = json.loads(value['__value__'])
            dtype = re.fullmatch(r'[<>=|]?[biufc](\d+)', inner['dtype'])
            if dtype is None:
                raise InvalidFixture('array')
            payload = base64.b64decode(inner['value'], validate=True)
            if len(payload) != count * int(dtype[1]):
                raise InvalidFixture('array')
        else:
            arrays = value['value'] if value['__dtype__'].startswith('complex') else [value['value']]
            if any(len(v) != count for v in arrays):
                raise InvalidFixture('array')
        return 1
    return sum(check_arrays(v) for v in value.values())


def inspect_fixture(payload):
    """Check only the framing and numeric layouts used in this fixture bundle."""
    if payload.startswith(b'\x1f\x8b'):
        try:
            payload = gzip.decompress(payload)
        except (EOFError, OSError):
            raise InvalidFixture('gzip') from None
    if not payload.startswith(b'##LARIX: 1.0 ') or not payload.endswith(b'##</Symbols>\n'):
        raise InvalidFixture('format')
    symbols = {}
    lines = iter(payload.decode('utf-8').splitlines())
    for line in lines:
        if line.startswith('<:') and line.endswith(':>'):
            try:
                symbols[line[2:-2]] = json.loads(next(lines))
            except (StopIteration, json.JSONDecodeError):
                raise InvalidFixture('json') from None
    count = check_arrays(symbols)
    for group in symbols.values():
        if isinstance(group, dict) and 'energy' in group and 'mu' in group:
            energy, mu = array_shape(group['energy']), array_shape(group['mu'])
            if energy != mu or energy is None or len(energy) != 1:
                raise InvalidFixture('mapping')
    for label, name in symbols.get('_xasgroups', {}).items():
        if label != '__class__' and name not in symbols:
            raise InvalidFixture('reference')
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--larch', action='store_true', help='Compare reopened sessions to pre-save observations.')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / 'manifest.json').read_text())
    for entry in manifest['files'] + manifest['assets']:
        path = root / entry['path']
        assert path.stat().st_size == entry['bytes'], f"Size differs: {entry['path']}"
        assert digest(path) == entry['sha256'], f"SHA-256 differs: {entry['path']}"
    expected_paths = {v['path'] for v in manifest['files']}
    actual_paths = {p.relative_to(root).as_posix() for p in (root / 'fixtures').rglob('*.larix')}
    assert expected_paths == actual_paths, 'Fixture inventory differs from manifest'
    sources = json.loads((root / 'sources/manifest.json').read_text())
    for entry in sources:
        assert digest(root / entry['path']) == entry['sha256'], entry['path']
    valid = [v for v in manifest['files'] if v['category'] == 'valid']
    arrays = sum(inspect_fixture((root / v['path']).read_bytes()) for v in valid)
    invalid = [v for v in manifest['files'] if v['category'] == 'invalid']
    for entry in invalid:
        try:
            inspect_fixture((root / entry['path']).read_bytes())
        except InvalidFixture as exc:
            assert exc.layer == entry['expected_failure_layer'], entry['path']
        else:
            raise AssertionError(f"Invalid case was accepted: {entry['path']}")
    print(f"Integrity OK: {len(manifest['files'])} fixtures and {len(manifest['assets'])} assets.")
    print(f'Layout checks OK: {arrays} array payloads and {len(invalid)} documented invalid cases.')
    if args.larch:
        from larch.io import read_session
        from fixturelib import compare, snapshot
        expectations = json.loads((root / 'expected.json').read_text())['sessions']
        assert set(expectations) == {v['path'] for v in valid}
        for entry in valid:
            log = io.StringIO()
            with redirect_stdout(log), redirect_stderr(log):
                session = read_session(str(root / entry['path']), clean_xasgroups=False)
            assert not log.getvalue(), f"{entry['path']}: {log.getvalue()}"
            expected = expectations[entry['path']]
            compare(snapshot(session.symbols), expected['symbols'], entry['path'])
            assert session.command_history == expected['command_history'], entry['path']
            assert session.config['Larix Version'] == '1.0', entry['path']
        print(f'Larch round trips OK: {len(valid)} valid sessions; all recorded arrays and metadata match.')
    print('Invalid cases were checked by the fixture inspector, not a rexafs or Larch rejection test.')


if __name__ == '__main__':
    main()
