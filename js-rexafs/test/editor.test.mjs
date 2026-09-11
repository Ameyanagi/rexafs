// Exercise the declarations from an npm tarball, as an editor sees an installed
// dependency. Runtime processing tests alone cannot catch missing exports/docs.
import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import ts from "typescript";

const root = fileURLToPath(new URL("../", import.meta.url));
const directory = mkdtempSync(join(tmpdir(), "rexafs-types-"));
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
function run(args, cwd) {
  const result = spawnSync(npm, args, { cwd, encoding: "utf8", shell: process.platform === "win32" });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  return result.stdout;
}
const packed = JSON.parse(run(["pack", "--json", "--pack-destination", directory], root))[0];
writeFileSync(join(directory, "package.json"), '{"type":"module","private":true}');
run(["install", "--ignore-scripts", "--no-audit", "--no-fund", join(directory, packed.filename)], directory);
process.on("exit", () => rmSync(directory, { recursive: true, force: true }));

for (const [entry, resolution] of [["rexafs", "NodeNext"], ["rexafs/node", "NodeNext"], ["rexafs/browser", "Bundler"]]) {
  test(`installed ${entry}: type checking, completion, signature help and hover (${resolution})`, () => {
    const filename = join(directory, `example-${entry.replaceAll("/", "-")}.ts`);
    let source = `import init, { Spectrum, AUTOBK, PrePostEdge, XrayFFTF, XrayFFTR,
      type FTWindow, type FFTGrid, type AUTOBKSolver, type AUTOBKClampScalePolicy,
      type AUTOBKOptions, type PrePostEdgeOptions, type XrayFFTFOptions, type XrayFFTROptions } from "${entry}";
await init();
const energy = new Float64Array([1, 2, 3]);
const spectrum = new Spectrum(energy, energy);
const options: AUTOBKOptions = { rbkg: 1, solver: "LinearDirect" };
const background = new AUTOBK(options);
const forward = new XrayFFTF({ grid: "Larch", window: "Hanning" });
const inverse = new XrayFFTR({ rmin: 1, rmax: 3 });
spectrum.set_normalization_method(new PrePostEdge()).set_background_method(background)
  .set_fft(forward).set_ifft(inverse).ifft();
const q = spectrum.q();
if (q !== undefined) q[0].toFixed(3);
background.rbkg;
spectrum.fft();
// @ts-expect-error misspelled option
new AUTOBK({ rbgk: 1 });
// @ts-expect-error invalid window
new XrayFFTF({ window: "Typo" });
// @ts-expect-error optional result must be narrowed
spectrum.chi()[0];
`;
    const options = {
      strict: true, exactOptionalPropertyTypes: true, noEmit: true, target: ts.ScriptTarget.ES2022,
      module: resolution === "Bundler" ? ts.ModuleKind.ESNext : ts.ModuleKind.NodeNext,
      moduleResolution: ts.ModuleResolutionKind[resolution], types: [],
    };
    const host = {
      getScriptFileNames: () => [filename], getScriptVersion: () => String(source.length),
      getScriptSnapshot: name => name === filename ? ts.ScriptSnapshot.fromString(source)
        : ts.sys.fileExists(name) ? ts.ScriptSnapshot.fromString(ts.sys.readFile(name)) : undefined,
      getCurrentDirectory: () => directory, getCompilationSettings: () => options,
      getDefaultLibFileName: opts => ts.getDefaultLibFilePath(opts),
      fileExists: name => name === filename || ts.sys.fileExists(name),
      readFile: ts.sys.readFile, readDirectory: ts.sys.readDirectory,
    };
    const service = ts.createLanguageService(host);
    try {
      const diagnostics = [...service.getCompilerOptionsDiagnostics(), ...service.getSyntacticDiagnostics(filename), ...service.getSemanticDiagnostics(filename)];
      assert.deepEqual(diagnostics.map(d => ts.flattenDiagnosticMessageText(d.messageText, "\n")), []);
      const hover = (text, offset = 0) => service.getQuickInfoAtPosition(filename, source.indexOf(text) + offset);
      assert.match(ts.displayPartsToString(hover("background.rbkg", 12).documentation), /angstroms.*Default: 1.0/);
      assert.match(ts.displayPartsToString(hover("spectrum.fft", 10).documentation), /2048/);
      const completeSource = source;
      const completion = suffix => {
        source = completeSource + suffix;
        const result = service.getCompletionsAtPosition(filename, source.length, {});
        assert.ok(result, suffix);
        return result.entries.map(e => e.name);
      };
      assert.ok(completion("\nspectrum.").includes("set_ifft"));
      assert.ok(completion("\nnew AUTOBK({ ").includes("clamp_lambda"));
      assert.ok(completion('\nnew XrayFFTF({ window: "').includes("KaiserBessel"));
      source = completeSource + '\nnew AUTOBK(';
      const signature = service.getSignatureHelpItems(filename, source.length, {});
      assert.ok(signature?.items.some(item => ts.displayPartsToString(item.parameters[0].displayParts).includes("AUTOBKOptions")));
    } finally { service.dispose(); }
  });
}
