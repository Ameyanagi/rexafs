"""Independent, JSON-friendly observations of in-memory Larch session values."""

import hashlib
import math
from datetime import datetime
from pathlib import Path


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def compare(actual, expected, path='session'):
    """Fail with the first differing location rather than dumping large spectra."""
    if isinstance(expected, dict) and isinstance(actual, dict):
        assert actual.keys() == expected.keys(), f'{path}: keys differ'
        for key in expected:
            compare(actual[key], expected[key], f'{path}/{key}')
    elif isinstance(expected, list) and isinstance(actual, list):
        assert len(actual) == len(expected), f'{path}: lengths differ'
        for index, (a, e) in enumerate(zip(actual, expected)):
            compare(a, e, f'{path}/{index}')
    else:
        assert actual == expected, f'{path}: {actual!r} != {expected!r}'


def snapshot(value):
    """Record values without using Larch's JSON encoder; hash every array element.

    Array hashes use C order and the original dtype converted to little endian.
    Complex samples are [real, imaginary] pairs. Array samples are flat indices.
    """
    import numpy as np
    from larch import Group, Journal
    from lmfit import Parameter, Parameters

    if isinstance(value, np.ndarray):
        flat = value.ravel()
        indices = sorted({0, flat.size // 2, flat.size - 1}) if flat.size else []
        little = value.astype(value.dtype.newbyteorder('<'), copy=False)
        return {
            "kind": "ndarray", "dtype": value.dtype.str, "shape": list(value.shape),
            "sha256_le_c": hashlib.sha256(little.tobytes(order='C')).hexdigest(),
            "samples": [{"index": i, "value": snapshot(flat[i].item())} for i in indices],
        }
    if isinstance(value, Journal):
        return {"kind": "Journal", "entries": snapshot(value.__getstate__())}
    if isinstance(value, Parameters):
        return {"kind": "Parameters", "values": {k: snapshot(v) for k, v in value.items()}}
    if isinstance(value, Parameter):
        return {"kind": "Parameter", **{
            k: snapshot(getattr(value, k))
            for k in ('name', 'value', 'vary', 'min', 'max', 'expr', 'stderr')
        }}
    if isinstance(value, Group):
        return {"kind": type(value).__name__, "attributes": {
            key: snapshot(getattr(value, key)) for key in dir(value)
            if not callable(getattr(value, key))
        }}
    if isinstance(value, dict):
        return {str(k): snapshot(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [snapshot(v) for v in value]
    if isinstance(value, datetime):
        return {"kind": "Datetime", "value": value.isoformat()}
    if isinstance(value, Path):
        return {"kind": "Path", "value": value.as_posix()}
    if isinstance(value, complex):
        return [value.real, value.imag]
    if isinstance(value, np.generic):
        return snapshot(value.item())
    if isinstance(value, float) and not math.isfinite(value):
        return str(value)
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    raise TypeError(f"Add an explicit snapshot rule for {type(value).__name__}")
