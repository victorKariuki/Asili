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
  const serverPath = config.get<string>("serverPath") ?? "pata";
  const serverArgs = config.get<string[]>("serverArgs") ?? ["mwalimu"];

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
