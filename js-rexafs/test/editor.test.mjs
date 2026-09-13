// Exercise the declarations from an npm tarball, as an editor sees an installed
// dependency. Runtime processing tests alone cannot catch missing exports/docs.
import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
// TypeScript 7 ships the native compiler; Microsoft provides the previous
// JavaScript language-service API through this compatibility package.
import ts from "@typescript/typescript6";

const root = fileURLToPath(new URL("../", import.meta.url));
const directory = mkdtempSync(join(tmpdir(), "rexafs-types-"));
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const compiler = fileURLToPath(new URL("../node_modules/typescript/bin/tsc", import.meta.url));
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
  test(`installed ${entry}: TypeScript 7 checking and editor completion, signatures and hover (${resolution})`, () => {
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
forward.kstep;
forward.dk;
spectrum.chir_real();
spectrum.fft();
spectrum.set_background_method;
// @ts-expect-error misspelled option
new AUTOBK({ rbgk: 1 });
// @ts-expect-error invalid window
new XrayFFTF({ window: "Typo" });
// @ts-expect-error optional result must be narrowed
spectrum.chi()[0];
`;
    const compilerOptions = {
      strict: true, exactOptionalPropertyTypes: true, noEmit: true, target: "ES2022",
      module: resolution === "Bundler" ? "ESNext" : "NodeNext",
      moduleResolution: resolution, types: [],
    };
    // Run the current native compiler on the same installed-package fixture,
    // including @ts-expect-error assertions for invalid options and results.
    writeFileSync(filename, source);
    const config = `${filename}.json`;
    writeFileSync(config, JSON.stringify({ compilerOptions, files: [filename] }));
    const checked = spawnSync(process.execPath, [compiler, "--project", config, "--pretty", "false"], {
      cwd: directory, encoding: "utf8",
    });
    assert.equal(checked.status, 0, checked.stdout + checked.stderr);
    const { options, errors } = ts.convertCompilerOptionsFromJson(compilerOptions, directory);
    assert.deepEqual(errors, []);
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
      const hoverText = (text, offset) => ts.displayPartsToString(hover(text, offset).documentation).replace(/\s+/g, " ");
      assert.match(hoverText("background.rbkg", 12), /angstroms.*Default: 1.0/);
      assert.match(hoverText("spectrum.fft", 10), /2048/);
      assert.match(hoverText("forward.kstep", 8), /infers the first spacing/);
      assert.match(hoverText("forward.dk", 8), /KaiserBessel.*shape parameter/);
      assert.match(hoverText("spectrum.chir_real", 10), /kstep\/sqrt\(pi\)/);
      assert.match(hoverText("type AUTOBKClampScalePolicy", 5), /optimization objective/);
      assert.match(hoverText("spectrum.set_background_method", 10), /undefined or null restores default AUTOBK settings/);
      assert.match(hoverText("spectrum.set_background_method", 10), /work in 0\.2\.4 and later/);
      assert.match(hoverText("spectrum.set_background_method", 10), /direct AUTOBK settings were added in 0\.2\.5/);
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
      assert.ok(signature?.items.some(item => ts.displayPartsToString(item.documentation).includes("recommended defaults")));
      assert.ok(signature?.items.some(item => ts.displayPartsToString(item.documentation).includes("Named options were added in 0.2.5")));
    } finally { service.dispose(); }
  });
}
