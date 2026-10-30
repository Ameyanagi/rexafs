# Published 0.2.15 consumer examples

Fresh consumer projects installed `rexafs==0.2.15` from PyPI and `rexafs@0.2.15`
from npm. The Python project used native Apple Silicon CPython 3.12.14 through
uv; the JavaScript project installed the package with Bun and ran it with Node
v26.12.1. Neither used a workspace package. The public package files were
separately [hash-verified](published-artifacts.md).

The examples from the [Python guide](../../../website/src/content/docs/docs/libraries/python.md)
and [TypeScript guide](../../../website/src/content/docs/docs/libraries/typescript.md)
read the same `618`-point `cu_150k.xmu` spectrum, normalize it,
remove the background and compute its Fourier transform. Each script is the
unmodified guide example followed by lines that write E₀ and the R and
Fourier-magnitude arrays to JSON for comparison. Both reported
E₀ = 8977.493 eV and 326 finite Fourier-magnitude points. The maximum absolute
difference was 4.93058e-13; the R grids differed by at most 0 Å. Comparison
required finite values, matching array lengths and absolute differences below
10⁻⁹.

This checks installation and the documented processing path. It does not establish
the physical interpretation of Fourier peaks or qualify the desktop GUI.

| Retained input | SHA-256 |
|---|---|
| Cu spectrum | `c309e53ec6b681024718d5c25694c6426818725974afd7b124a2f076be618cf2` |
| Python example with JSON output | `83d3b96687d274059a6539ba4074e632dae1b37f29185a42d2b9d60809242417` |
| JavaScript example with JSON output | `8fcf1704bcfcc24bcfd9e4e554fec5f7592e84d588ca84a555215e58bb8e64d3` |
