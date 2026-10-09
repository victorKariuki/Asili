// Runs Asili programs through the browser build of asili-wasm under Node: native code is a
// wasm module instantiated beside it, as the playground does. Usage:
//   node run_node.mjs <pkg-dir> <file.as>...   (pkg-dir: `wasm-bindgen --target nodejs` output)
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import path from "node:path";

// The loader the playground defines: instantiate native code against the host's memory and
// function table, in table slots grown for it.
globalThis.asili_nguvu_load = (bytes, count, memory, table) => {
  const base = table.grow(count);
  new WebAssembly.Instance(new WebAssembly.Module(bytes), { env: { memory, table, base } });
  return base;
};

const [pkg, ...files] = process.argv.slice(2);
const require = createRequire(import.meta.url);
const asili = require(path.resolve(pkg, "asili_wasm.js"));
let failed = 0;
for (const file of files) {
  try {
    asili.run(readFileSync(file, "utf8"));
  } catch (e) {
    failed++;
    console.error(`${file}: ${e}`);
  }
}
process.exit(failed ? 1 : 0);
