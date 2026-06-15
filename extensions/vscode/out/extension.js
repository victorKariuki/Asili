"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = require("path");
const fs = require("fs");
const vscode_1 = require("vscode");
const node_1 = require("vscode-languageclient/node");
let client;
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
}
function deactivate() {
    return client?.stop();
}
//# sourceMappingURL=extension.js.map