---
title: "Troubleshooting"
description: "Resolve common installation, data and processing problems."
audience: user
---

| Symptom | Check |
|---|---|
| Import remains pending | Select the main signal, assign columns, confirm axis units and revalidate. |
| The edge is at the wrong energy | Check eV versus keV and calibration; an automatic edge estimate is not a reference calibration. |
| Input arrays are rejected | Match lengths, remove non-finite values deliberately, and provide a strictly increasing energy axis. |
| `chi()`, `r()` or a window is unavailable | Run the required stage and handle `None`/`undefined` before using the result. |
| Python import/completion uses the wrong package | Select the same virtual environment in your terminal, editor and Jupyter kernel. |
| Constructor arguments are rejected | Check the version: stable 0.2.4 uses zero-argument settings constructors. Next API is unreleased. |
| Browser objects fail during construction | Await Wasm initialization and make sure the bundler serves the `.wasm` asset. |
| A TypeScript object fails after cleanup | `free()` ends its lifetime. Create a new object before using the API again. |
| A linked project cannot find data | Restore the relative folder layout, or use an embedded project when sharing. |
| A fit converges with implausible parameters | Inspect ranges, model paths, constraints, residuals, bounds and parameter correlations. |
| Linux does not open a window | Use a graphical session and the required GTK/font/Vulkan runtime and driver. |

## Recover a project

The previous save is retained as `.rxs.bak`. Copy it to a new `.rxs` filename
and open the copy. Keep independent backups of important experiments; a previous
save on the same disk is not an archival backup.

## Ask for help

[Open a GitHub issue](https://github.com/Ameyanagi/rexafs/issues) with the rexafs
version, OS, interface, exact error and a small reproducible example. Share only
data you are able to distribute. Include processing settings and units when
reporting a numerical difference; a screenshot alone may not identify its cause.
