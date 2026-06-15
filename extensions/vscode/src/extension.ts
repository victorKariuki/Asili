import * as path from "path";
import * as fs from "fs";
import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

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
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
