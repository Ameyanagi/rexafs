// Run against an installed wheel: REXAFS_PYTHON=/path/to/venv/bin/python npm run test:python-editor
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const python = process.env.REXAFS_PYTHON;
assert.ok(python, "Set REXAFS_PYTHON to the interpreter with the newly built wheel installed");
const installed = spawnSync(python, ["-c", "import rexafs; from pathlib import Path; p=Path(rexafs.__file__).parent; assert (p/'py.typed').is_file(); assert (p/'__init__.pyi').is_file(); print(p.parent)"], { encoding: "utf8" });
assert.equal(installed.status, 0, installed.stderr);
const directory = mkdtempSync(join(tmpdir(), "rexafs-python-editor-"));
const rootUri = pathToFileURL(directory).href;
writeFileSync(join(directory, "pyrightconfig.json"), JSON.stringify({
  typeCheckingMode: "strict", pythonVersion: "3.10",
}));
const valid = `from rexafs import AUTOBK, Spectrum, PeakFit, XrayFFTF, XrayFFTR, FTWindow
from rexafs.io import parse_measurement, SpectrumMapping
measurement = parse_measurement("energy,mu\\n7100,1\\n7101,2")
mapping: SpectrumMapping = {"energy_column":0,"energy":{"kind":"offset_ev","offset_ev":20000},"signal":{"kind":"direct","column":1}}
energy, mu = measurement.arrays(mapping=mapping)
energy, mu = measurement.arrays(energy="energy", mu="mu")
measurement.arrays(energy=0, i0="I0", iff=["IFF1", 3], energy_unit="keV")
named_mapping: SpectrumMapping = {"energy_column":"energy","energy":{"kind":"ev"},"signal":{"kind":"direct","column":"mu"}}
measurement.spectrum(mapping=named_mapping)
print(energy[0], mu[0], measurement.document["format"])
archived: list[float | None] | None = measurement.document["datasets"][0]["imaginary"]
print(archived, measurement.document["metadata"].get("larix.session_text"))
window: FTWindow = "Hanning"
spectrum = Spectrum([1., 2.], [1., 2.])
spectrum.set_background_method(AUTOBK(rbkg=1.2)).set_fft(XrayFFTF(window=window)).set_ifft(XrayFFTR(rmin=1.0)).ifft()
scalar = spectrum.measure("mean", (-20., 30.), space="flat")
print(scalar.value, scalar.standard_error, scalar.range, scalar.to_json())
spectrum.measure("point", 10.)
peak = PeakFit((-20., 40.)).gaussian("p1", center=5, area=2, fwhm=3).linear_baseline()
fit = spectrum.fit_peaks(peak)
print(fit.parameters["p1_center"], fit.components[0].area, fit.to_json())
for row in peak.fit_batch([spectrum]):
    if row.result is not None:
        print(row.result.objective)
chi = spectrum.chi()
if chi is not None:
    print(chi[0])
`;
writeFileSync(join(directory, "valid.py"), valid);
writeFileSync(join(directory, "invalid.py"), `from rexafs import AUTOBK, Spectrum, XrayFFTF
AUTOBK(rbgk=1.0)
XrayFFTF(window="Typo")
Spectrum([1., 2.], [1., 2.]).chi()[0]
`);
const checker = fileURLToPath(new URL("../node_modules/pyright/index.js", import.meta.url));
const checked = spawnSync(process.execPath, [checker, "--project", directory, "--pythonpath", python, "--outputjson"], { encoding: "utf8" });
const diagnostics = JSON.parse(checked.stdout).generalDiagnostics;
assert.deepEqual(diagnostics.filter(d => d.file.endsWith("valid.py") && !d.file.endsWith("invalid.py")), []);
assert.deepEqual(diagnostics.filter(d => d.severity === "error").map(d => d.range.start.line).sort(), [1, 2, 3]);
const script = fileURLToPath(new URL("../node_modules/pyright/dist/pyright-langserver.js", import.meta.url));
const child = spawn(process.execPath, [script, "--stdio"], { cwd: directory });
let buffer = Buffer.alloc(0), id = 0;
const pending = new Map();
const deadline = setTimeout(() => { child.kill(); throw new Error("Python language-server test timed out"); }, 60000);
function send(message) {
  const body = JSON.stringify({ jsonrpc: "2.0", ...message });
  child.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}
function request(method, params) {
  return new Promise((resolve, reject) => {
    const sequence = ++id; pending.set(sequence, { resolve, reject }); send({ id: sequence, method, params });
  });
}
child.stdout.on("data", chunk => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd < 0) return;
    const length = Number(/Content-Length: (\d+)/i.exec(buffer.subarray(0, headerEnd).toString())[1]);
    if (buffer.length < headerEnd + 4 + length) return;
    const message = JSON.parse(buffer.subarray(headerEnd + 4, headerEnd + 4 + length));
    buffer = buffer.subarray(headerEnd + 4 + length);
    if (message.method && message.id !== undefined) send({ id: message.id, result: null });
    else if (pending.has(message.id)) {
      const waiter = pending.get(message.id); pending.delete(message.id);
      if (message.error) waiter.reject(new Error(JSON.stringify(message.error))); else waiter.resolve(message.result);
    }
  }
});
child.on("exit", code => { for (const waiter of pending.values()) waiter.reject(new Error(`Language server exited: ${code}`)); });
child.stderr.on("data", chunk => process.stderr.write(chunk));
try {
  await request("initialize", { processId: process.pid, rootUri, capabilities: {}, initializationOptions: {} });
  send({ method: "initialized", params: {} });
  send({ method: "workspace/didChangeConfiguration", params: { settings: { python: { pythonPath: resolve(python) } } } });
  const text = `from rexafs import AUTOBK, Spectrum, XrayFFTF, XrayFFTR
background = AUTOBK(rbkg=1.2)
spectrum = Spectrum([1., 2., 3.], [1., 2., 3.])
spectrum.set_background_method(background).set_ifft(XrayFFTR(rmin=1.0))
background.rbkg
spectrum.fft
spectrum.
AUTOBK(
XrayFFTF(window="")
`;
  const uri = pathToFileURL(join(directory, "example.py")).href;
  send({ method: "textDocument/didOpen", params: { textDocument: { uri, languageId: "python", version: 1, text } } });
  const position = (line, character) => ({ textDocument: { uri }, position: { line, character } });
  const hover = await request("textDocument/hover", position(4, 13));
  assert.match(JSON.stringify(hover), /angstroms.*Default: 1.0/);
  const stage = await request("textDocument/hover", position(5, 11));
  assert.match(JSON.stringify(stage), /2048/);
  const methods = await request("textDocument/completion", position(6, 9));
  assert.ok((methods.items ?? methods).some(item => item.label === "set_ifft"));
  const keywords = await request("textDocument/completion", position(7, 7));
  assert.ok((keywords.items ?? keywords).some(item => item.label.startsWith("clamp_lambda")));
  const choices = await request("textDocument/completion", position(8, 17));
  assert.ok((choices.items ?? choices).some(item => item.label.includes("KaiserBessel")));
  const signature = await request("textDocument/signatureHelp", position(7, 7));
  assert.match(JSON.stringify(signature), /clamp_lambda.*0\.001/);
  const readerText = 'from rexafs.io import parse_measurement\nmeasurement = parse_measurement("energy,mu\\n7100,1")\nmeasurement.arrays\nmeasurement.\nmeasurement.arrays(';
  const readerUri = pathToFileURL(join(directory,"reader.py")).href;
  send({method:"textDocument/didOpen",params:{textDocument:{uri:readerUri,languageId:"python",version:1,text:readerText}}});
  const readerPosition=(line,character)=>({textDocument:{uri:readerUri},position:{line,character}});
  assert.match(JSON.stringify(await request("textDocument/hover",readerPosition(2,15))),/eV/);
  const readerMethods=await request("textDocument/completion",readerPosition(3,12));
  assert.ok((readerMethods.items ?? readerMethods).some(item=>item.label==="select_datasets"));
  assert.match(JSON.stringify(await request("textDocument/signatureHelp",readerPosition(4,19))),/SpectrumMapping/);
  const columnKeywords=await request("textDocument/completion",readerPosition(4,19));
  for (const name of ["energy", "mu", "i0", "it", "iff", "energy_unit"]) {
    assert.ok((columnKeywords.items ?? columnKeywords).some(item=>item.label.startsWith(`${name}=`)), name);
  }
  assert.match(JSON.stringify(await request("textDocument/hover",readerPosition(2,15))),/case-sensitive/);
  const scalarText = 'from rexafs import Spectrum\ns = Spectrum([0., 1.], [1., 2.])\ns.measure\ns.measure("mean", (0., 1.), \n';
  const scalarUri = pathToFileURL(join(directory, "scalar.py")).href;
  send({method:"textDocument/didOpen",params:{textDocument:{uri:scalarUri,languageId:"python",version:1,text:scalarText}}});
  const scalarPosition = (line, character) => ({textDocument:{uri:scalarUri},position:{line,character}});
  assert.match(JSON.stringify(await request("textDocument/hover",scalarPosition(2,5))), /private copy/);
  assert.match(JSON.stringify(await request("textDocument/signatureHelp",scalarPosition(3,27))), /space/);
  const peakText = 'from rexafs import Spectrum, PeakFit\ns = Spectrum([0., 1.], [1., 2.])\np = PeakFit((-20., 40.))\ns.fit_peaks\np.fit_batch\np.gaussian("p", \n';
  const peakUri = pathToFileURL(join(directory, "peaks.py")).href;
  send({ method: "textDocument/didOpen", params: { textDocument: { uri: peakUri, languageId: "python", version: 1, text: peakText } } });
  const peakPosition = (line, character) => ({ textDocument: { uri: peakUri }, position: { line, character } });
  assert.match(JSON.stringify(await request("textDocument/hover", peakPosition(3, 7))), /E0-relative/);
  assert.match(JSON.stringify(await request("textDocument/hover", peakPosition(4, 7))), /one outcome per input/);
  assert.match(JSON.stringify(await request("textDocument/signatureHelp", peakPosition(5, 16))), /fwhm/);
  const peakKeywords = await request("textDocument/completion", peakPosition(5, 16));
  assert.ok((peakKeywords.items ?? peakKeywords).some(x => x.label.startsWith("fwhm")));
  console.log("Installed Python wheel: property/method hovers, member/keyword/literal completion and signature defaults passed");
  await request("shutdown", null);
  send({ method: "exit", params: null });
} finally {
  clearTimeout(deadline); child.kill(); rmSync(directory, { recursive: true, force: true });
}
