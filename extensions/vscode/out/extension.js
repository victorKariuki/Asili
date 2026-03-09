"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode_1 = require("vscode");
const node_1 = require("vscode-languageclient/node");
let client;
function activate(context) {
    const config = vscode_1.workspace.getConfiguration("asili");
    const serverPath = config.get("serverPath") ?? "pata";
    const serverArgs = config.get("serverArgs") ?? ["mwalimu"];
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