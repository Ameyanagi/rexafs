# Python ABI3 feasibility pilot

**Local experiment only.** This wheel was not published and does not qualify
the 0.2.5 release. ABI3 is CPython's stable binary interface, which permits one
extension binary to serve several Python versions on the same platform.
[PyO3 documents its restrictions](https://pyo3.rs/v0.29.2/building-and-distribution.html#py_limited_apiabi3abi3t).

On 13 September 2026, source exported from immutable `v0.2.5` commit
`50d59e2147e12a98ace9a9e17df872a910e9ca24` was built in a fresh temporary Cargo
target directory. macOS 26.5.1 ARM64, Rust 1.98.1 and maturin 1.12.6 were used:

```sh
maturin build --release --locked --manifest-path py-rexafs/Cargo.toml \
  --features pyo3/abi3-py310 --interpreter BUILD_ENV/bin/python --out WHEELS
```

`BUILD_ENV` contained CPython 3.12.12; `WHEELS` was an isolated output directory.
The cold build command took **185.7 seconds**. It produced
`rexafs-0.2.5-cp310-abi3-macosx_11_0_arm64.whl` (1,428,470 bytes), SHA-256:

```text
403bb7598863c7e6ea9bb4f261a04128294c60c06ae68b15b31638fe30e58e9e
```

Five fresh environments installed that identical wheel without recompilation.
Each imported extension was verified as the environment's `_core.abi3.so`.
The unchanged [API suite](https://github.com/Ameyanagi/rexafs/blob/50d59e2147e12a98ace9a9e17df872a910e9ca24/py-rexafs/tests/test_api.py) passed:

| CPython | NumPy | Tests |
| --- | --- | --- |
| 3.10.20 | 2.2.6 | 14 passed |
| 3.11.14 | 2.4.6 | 14 passed |
| 3.12.12 | 2.5.3 | 14 passed |
| 3.13.11 | 2.5.3 | 14 passed |
| 3.14.2 | 2.5.3 | 14 passed |

Test processes took 0.132–0.183 seconds each, including interpreter startup.
These are local validation timings, not processing benchmarks or CI forecasts.

Coverage is limited to macOS ARM64, GIL-enabled interpreters (using CPython's
global interpreter lock) and their latest compatible NumPy versions. ABI3 does
not cover free-threaded Python 3.13/3.14 or independently guarantee
[NumPy compatibility](https://numpy.org/doc/stable/dev/depending_on_numpy.html).

A future release could build four platform wheels and retain all 20 runtime
checks. First qualify Linux x64, Windows x64 and Intel macOS; test minimum
supported NumPy versions; retain editor, numerical and source-distribution
checks; and introduce explicit ABI3 artifact inventories and publication
contracts. Preserve the immutable 0.2.5 artifacts.

Raw logs, wheel metadata, hashes and environment records remain locally under
`/tmp/rexafs-abi3-feasibility/` (`build.json`, `wheel.json`, `tests.json` and
per-interpreter logs). They are not public release attachments.
