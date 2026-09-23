# Published 0.2.14 consumer examples

Fresh consumer projects installed `rexafs==0.2.14` from PyPI and `rexafs@0.2.14`
from npm. The Python project used native Apple Silicon CPython 3.12.12;
the JavaScript project used Node v24.19.0. Neither used a workspace package.
The public package files were separately [hash-verified](published-artifacts.md).

The unmodified examples from the [Python guide](../../../website/src/content/docs/docs/libraries/python.md)
and [TypeScript guide](../../../website/src/content/docs/docs/libraries/typescript.md)
read the same `618`-point `cu_150k.xmu` spectrum, normalize it,
remove the background and compute its Fourier transform. Both reported
E₀ = 8977.493 eV and 326 finite
Fourier-magnitude points. The maximum absolute difference was
4.93058e-13; the R grids differed by at most
0 Å. Comparison required finite values,
matching array lengths and absolute differences below 10⁻⁹.

This checks installation and the documented processing path. It does not establish
the physical interpretation of Fourier peaks or qualify the desktop GUI.

| Retained input | SHA-256 |
|---|---|
| Cu spectrum | `c309e53ec6b681024718d5c25694c6426818725974afd7b124a2f076be618cf2` |
| Python example | `dc7d0f9dacfe295b030b163ad1854ae6bd14107c66e404b30a6ab417506bb633` |
| JavaScript example | `3952d21842840f42f4dab8b1178422a13a396840a28268fc855119019cb9e24a` |
