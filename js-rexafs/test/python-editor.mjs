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
const valid = `from rexafs import AUTOBK, Spectrum, XrayFFTF, XrayFFTR, FTWindow
window: FTWindow = "Hanning"
spectrum = Spectrum([1., 2.], [1., 2.])
spectrum.set_background_method(AUTOBK(rbkg=1.2)).set_fft(XrayFFTF(window=window)).set_ifft(XrayFFTR(rmin=1.0)).ifft()
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
  console.log("Installed Python wheel: property/method hovers, member/keyword/literal completion and signature defaults passed");
  await request("shutdown", null);
  send({ method: "exit", params: null });
} finally {
  clearTimeout(deadline); child.kill(); rmSync(directory, { recursive: true, force: true });
}
