// Bundles src/extension.ts + its runtime dependencies (vscode-languageclient and its own
// transitive deps) into a single out/extension.js. Avoids shipping node_modules in the .vsix at
// all -- the modern, recommended approach for VS Code extensions, and sidesteps vsce's own
// npm-list-based dependency detection (which broke under this project's UUID-shaped mount path,
// see docs/design -- npm 11's secret-redaction feature mangles paths matching a UUID pattern in
// `npm list`'s text output, which is exactly what vsce parses to find node_modules to bundle).
const esbuild = require("esbuild");

const watch = process.argv.includes("--watch");

const options = {
  entryPoints: ["src/extension.ts"],
  bundle: true,
  outfile: "out/extension.js",
  external: ["vscode"], // provided by the extension host at runtime, never bundle it
  format: "cjs",
  platform: "node",
  target: "node16",
  sourcemap: true,
  minify: !watch,
};

async function main() {
  if (watch) {
    const ctx = await esbuild.context(options);
    await ctx.watch();
    console.log("watching...");
  } else {
    await esbuild.build(options);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
