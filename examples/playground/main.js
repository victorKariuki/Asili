// Asili Playground — loads the `asili-wasm` (wasm-browser) build and runs source from a
// CodeMirror 6 editor entirely client-side. No LSP wiring here yet (diagnostics/hover/format
// via wasm are a follow-up — see README.md "Baadaye / Next steps").

import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from "https://esm.sh/@codemirror/view@6";
import { EditorState } from "https://esm.sh/@codemirror/state@6";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "https://esm.sh/@codemirror/commands@6";
import { StreamLanguage, syntaxHighlighting, defaultHighlightStyle, indentOnInput, bracketMatching } from "https://esm.sh/@codemirror/language@6";
import { oneDark } from "https://esm.sh/@codemirror/theme-one-dark@6";

import init, { run } from "./pkg/asili_wasm.js";

// Keywords mirrored from pata/lsp/src/hover.rs's KEYWORDS list — keep in sync if that list grows.
const KEYWORDS = new Set([
  "leta", "kazi", "umbo", "sifa", "shughuli", "ya", "weka", "thabiti", "rejesha", "ikiwa",
  "vinginevyo", "kwa", "wakati", "linganisha", "vunja", "endelea", "lebo", "tupa", "jaribu",
  "kama", "azima", "azima_tenda", "umma", "katika", "kutoka", "au_ikiwa", "chapisha", "paparika",
  "hadi", "kweli", "sikweli", "kwa_ajili", "kagua",
]);
const BUILTIN_TYPES = new Set([
  "Namba", "Neno", "Tupu", "Boolean", "Orodha", "Ramani", "Seti", "Tokeo", "Chaguo", "Self",
  "Namba_Kuu", "Namba_Sahihi", "Biti8",
]);

// Minimal StreamLanguage-based highlighting mode for Asili — comments (#, //), strings, numbers,
// keywords and built-in type names. Not a real parser (no LSP/diagnostics), just enough to make
// the editor pleasant to read.
const asiliMode = {
  startState() {
    return { inBlockComment: false };
  },
  token(stream) {
    if (stream.eatSpace()) return null;
    if (stream.match("#")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.match("//")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.match('"')) {
      while (!stream.eol()) {
        if (stream.next() === "\\") stream.next();
        else if (stream.string[stream.pos - 1] === '"') break;
      }
      return "string";
    }
    if (stream.match(/^-?\d+(\.\d+)?/)) return "number";
    const word = stream.match(/^[A-Za-z_][A-Za-z0-9_]*/);
    if (word) {
      const w = word[0];
      if (KEYWORDS.has(w)) return "keyword";
      if (BUILTIN_TYPES.has(w)) return "typeName";
      if (/^[A-Z]/.test(w)) return "typeName";
      return null;
    }
    stream.next();
    return null;
  },
};

const state0 = () => EditorState.create({
  doc: "",
  extensions: [
    lineNumbers(),
    highlightActiveLine(),
    highlightActiveLineGutter(),
    history(),
    indentOnInput(),
    bracketMatching(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    StreamLanguage.define(asiliMode),
    keymap.of([indentWithTab, ...defaultKeymap, ...historyKeymap]),
    oneDark,
    EditorView.lineWrapping,
  ],
});

const editor = new EditorView({
  state: state0(),
  parent: document.getElementById("editor"),
});

const outputEl = document.getElementById("output");
const statusEl = document.getElementById("status");
const runBtn = document.getElementById("run-btn");
const clearBtn = document.getElementById("clear-btn");
const samplePicker = document.getElementById("sample-picker");

function setStatus(text, kind) {
  statusEl.textContent = text;
  statusEl.className = "status" + (kind ? ` ${kind}` : "");
}

function appendOutput(text, isErr) {
  const span = document.createElement("span");
  if (isErr) span.className = "line-err";
  span.textContent = text + "\n";
  outputEl.appendChild(span);
  outputEl.scrollTop = outputEl.scrollHeight;
}

// Route the wasm build's console.log/console.error (see core/evaluator/src/platform.rs's
// `wasm-browser` shim) into the output pane, in addition to the real devtools console.
const realLog = console.log.bind(console);
const realError = console.error.bind(console);
console.log = (...args) => {
  appendOutput(args.map(String).join(" "), false);
  realLog(...args);
};
console.error = (...args) => {
  appendOutput(args.map(String).join(" "), true);
  realError(...args);
};

async function loadSamples() {
  const res = await fetch("samples/manifest.json");
  const manifest = await res.json();
  for (const sample of manifest) {
    const opt = document.createElement("option");
    opt.value = sample.file;
    opt.textContent = sample.label;
    samplePicker.appendChild(opt);
  }
  if (manifest.length > 0) {
    await loadSample(manifest[0].file);
  }
}

async function loadSample(file) {
  const res = await fetch(`samples/${file}`);
  const text = await res.text();
  editor.setState(state0());
  editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: text } });
}

samplePicker.addEventListener("change", () => loadSample(samplePicker.value));
clearBtn.addEventListener("click", () => {
  outputEl.textContent = "";
});

// Fetched as base64 text (pkg/asili_wasm_bg.wasm.b64), not the raw .wasm binary, so this also
// works when served by examples/playground/src/kuu.as's Asili static server — its soma_faili
// only reads valid-UTF-8 text files (no raw-bytes file API yet).
async function loadWasmBytes() {
  const res = await fetch("pkg/asili_wasm_bg.wasm.b64");
  const b64 = (await res.text()).trim();
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

let wasmReady = loadWasmBytes().then((bytes) => init({ module_or_path: bytes })).then(() => {
  setStatus("Tayari (ready)", "ok");
  runBtn.disabled = false;
});

runBtn.disabled = true;
setStatus("Inapakia wasm…");

runBtn.addEventListener("click", async () => {
  await wasmReady;
  const source = editor.state.doc.toString();
  outputEl.textContent = "";
  runBtn.disabled = true;
  setStatus("Inaendesha…");
  try {
    run(source);
    setStatus("Imekamilika", "ok");
  } catch (e) {
    setStatus("Kosa", "error");
    appendOutput(String(e && e.message ? e.message : e), true);
  } finally {
    runBtn.disabled = false;
  }
});

// Ctrl/Cmd+Enter runs the current source.
window.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
    e.preventDefault();
    runBtn.click();
  }
});

loadSamples();
