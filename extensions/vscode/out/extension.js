"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = require("path");
const fs = require("fs");
const vscode_1 = require("vscode");
const node_1 = require("vscode-languageclient/node");
let client;
let testTerminal;
/// Walk up from `fileDir` looking for the nearest `pata.toml` — that directory is the
/// project root `pata jaribu` needs to run from. Falls back to `fileDir` itself if none is
/// found (e.g. a loose `.as` file with no project), matching how `pata-lsp`'s own workspace
/// resolution degrades gracefully in the same situation.
function findProjectRoot(fileDir) {
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
function activate(context) {
    const config = vscode_1.workspace.getConfiguration("asili");
    // Prefer the bundled binary, fall back to user-configured path or global pata.
    const bundled = context.asAbsolutePath(path.join("bin", "pata-lsp"));
    const hasBundled = fs.existsSync(bundled);
    const serverPath = config.get("serverPath") ?? (hasBundled ? bundled : "pata");
    const serverArgs = config.get("serverArgs") ?? (hasBundled ? [] : ["mwalimu"]);
    const executable = {
        command: serverPath,
        args: serverArgs,
    };
    const serverOptions = executable;
    const clientOptions = {
        documentSelector: [{ scheme: "file", language: "asili" }],
    };
    client = new node_1.LanguageClient("asiliLsp", "Asili (Mwalimu)", serverOptions, clientOptions);
    client.start();
    // Backing command for the "▶ Run Test" code lens pata-lsp publishes above every
    // #[jaribio] function (see pata/lsp/src/symbols.rs::test_code_lenses). Reuses one
    // terminal across runs rather than spawning a new one per test click.
    context.subscriptions.push(vscode_1.commands.registerCommand("asili.runTest", (uriStr, testName) => {
        const fileDir = path.dirname(vscode_1.Uri.parse(uriStr).fsPath);
        const root = findProjectRoot(fileDir);
        // Deliberately separate from asili.serverPath (which points at the LSP server, e.g.
        // a bundled pata-lsp with no sibling pata CLI binary) rather than derived from it.
        const pataPath = config.get("cliPath") ?? "pata";
        if (!testTerminal || testTerminal.exitStatus !== undefined) {
            testTerminal = vscode_1.window.createTerminal("Asili Tests");
        }
        testTerminal.show(true);
        testTerminal.sendText(`cd "${root}" && ${pataPath} jaribu --filter "${testName}"`);
    }));
}
function deactivate() {
    return client?.stop();
}
//# sourceMappingURL=extension.js.map