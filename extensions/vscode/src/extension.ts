import * as path from "path";
import * as fs from "fs";
import { commands, window, workspace, ExtensionContext, Terminal, Uri } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let testTerminal: Terminal | undefined;

/**
 * Walk up from `fileDir` looking for the nearest `pata.toml` — that directory is the
 * project root `pata jaribu` needs to run from.
 *
 * Falls back to `fileDir` itself if none is found (e.g. a loose `.as` file with no
 * project), matching how `pata-lsp`'s own workspace resolution degrades gracefully in
 * the same situation.
 *
 * @param fileDir - Directory to start the upward search from.
 * @returns The nearest ancestor directory containing a `pata.toml`, or `fileDir` if none exists.
 */
function findProjectRoot(fileDir: string): string {
  let dir = fileDir;
  for (;;) {
    if (fs.existsSync(path.join(dir, "pata.toml"))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return fileDir;
    }
    dir = parent;
  }
}

/**
 * Extension entry point, called once by VS Code when the extension activates
 * (on opening a `.as`/`.asi`/`pata.toml`/`pata.lock` file, per `package.json`'s
 * `activationEvents`/language contributions).
 *
 * Starts the Mwalimu language client (preferring the bundled `pata-lsp` binary under
 * `bin/`, falling back to `asili.serverPath`/a global `pata` on `PATH`) and registers
 * the `asili.runTest` command backing the "▶ Run Test" code lens `pata-lsp` publishes
 * above every `#[jaribio]` function.
 *
 * @param context - The extension context VS Code provides; used to resolve the bundled
 * binary path and to register disposables.
 */
export function activate(context: ExtensionContext): void {
  const config = workspace.getConfiguration("asili");

  // Prefer the bundled binary, fall back to user-configured path or global pata.
  const bundled = context.asAbsolutePath(path.join("bin", "pata-lsp"));
  const hasBundled = fs.existsSync(bundled);

  const serverPath = config.get<string>("serverPath") ?? (hasBundled ? bundled : "pata");
  const serverArgs = config.get<string[]>("serverArgs") ?? (hasBundled ? [] : ["mwalimu"]);

  const executable: Executable = {
    command: serverPath,
    args: serverArgs,
  };

  const serverOptions: ServerOptions = executable;
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "asili" }],
  };

  client = new LanguageClient(
    "asiliLsp",
    "Asili (Mwalimu)",
    serverOptions,
    clientOptions
  );

  client.start();

  // Backing command for the "▶ Run Test" code lens pata-lsp publishes above every
  // #[jaribio] function (see pata/lsp/src/symbols.rs::test_code_lenses). Reuses one
  // terminal across runs rather than spawning a new one per test click.
  context.subscriptions.push(
    commands.registerCommand("asili.runTest", (uriStr: string, testName: string) => {
      const fileDir = path.dirname(Uri.parse(uriStr).fsPath);
      const root = findProjectRoot(fileDir);
      // Deliberately separate from asili.serverPath (which points at the LSP server, e.g.
      // a bundled pata-lsp with no sibling pata CLI binary) rather than derived from it.
      const pataPath = config.get<string>("cliPath") ?? "pata";

      if (!testTerminal || testTerminal.exitStatus !== undefined) {
        testTerminal = window.createTerminal("Asili Tests");
      }
      testTerminal.show(true);
      testTerminal.sendText(`cd "${root}" && ${pataPath} jaribu --filter "${testName}"`);
    })
  );
}

/**
 * Extension shutdown hook, called by VS Code on deactivation. Stops the running
 * language client (if one was started) so the `pata mwalimu`/`pata-lsp` process
 * doesn't linger after the extension unloads.
 *
 * @returns The client's own stop promise, or `undefined` if no client was ever started.
 */
export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
