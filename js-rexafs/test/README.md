# Processing parity fixture

`ru-reference.json` records the default native rexafs pipeline on `Ru_QAS.dat`
(E0, grid lengths, and samples at indices 0, 20, 50 and 100). It was captured before
the September 2026 dependency upgrade to detect drift in the Wasm bindings and
upgraded numerical stack. Automatic prerequisite execution is separately compared with the explicit
staged Rust pipeline. These are software regression checks, not experimental validation.

## Editor contract checks

`npm ci && npm run build && npm test` also packs/installs the actual npm tarball
in a temporary application. `editor.test.mjs` checks root/Node/browser exports
with the pinned TypeScript 7.0.2 native compiler, including strict type checking
and invalid-option errors. The `@typescript/native` alias provides `tsc`;
the `typescript` alias uses `@typescript/typescript6` 6.0.2 (TypeScript 6.0.3 in
the lockfile) for the JavaScript language-service API and `tsc6`. All member,
options and string completions, signature help and parameter/method hover checks
remain enabled. These aliases follow Microsoft's
[compatibility guidance](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-6-0):
TypeScript 7 does not expose the earlier JavaScript language-service API.

For Python, first build and install the wheel into a virtual environment, then:

```bash
REXAFS_PYTHON=/absolute/path/to/venv/bin/python npm run test:python-editor
```

This starts the pinned Pyright language server against the **installed package**,
checks packaged `py.typed`/stubs, and requests completion, signature help and
hover through LSP. The release workflow runs it against a freshly installed
Python wheel. Both checks are needed: numerical runtime tests cannot detect
missing editor metadata.
