import * as vscode from "vscode";
import * as net from "node:net";
import * as os from "node:os";
import * as path from "node:path";
import {homeKey, LineDecoder, type ProfileEvent} from "./protocol";
const pipe = "\\\\.\\pipe\\CodexAccountManager";
export function activate(context: vscode.ExtensionContext) {
  const output = vscode.window.createOutputChannel("Codex Account Manager");
  const bar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 90);
  bar.command = "codexAccountManager.showStatus";
  let socket: net.Socket | undefined;
  let retry: ReturnType<typeof setTimeout> | undefined;
  let disposed = false, reconnectDelay = 500, connected = false, reloading = false;
  let blocked = false;
  let profile = "";
  const config = () => vscode.workspace.getConfiguration("codexAccountManager");
  const key = homeKey(process.env.CODEX_HOME || path.join(os.homedir(),".codex"));
  const stateKey = `profile:${key}`;
  const debug = (category: "CONNECTED" | "DISCONNECTED" | "INVALID_FRAME" | "REFRESH_FAILED") => { if(config().get("debugLogging",false)) output.appendLine(category); };
  function render() {
    bar.text = connected ? `$(account) Codex: ${profile || "No profile"}` : "$(plug) Codex Manager";
    bar.tooltip = connected ? "Local Codex Account Manager connected" : blocked ? "Conflicting account switcher detected" : "Waiting for Codex Account Manager";
    if(config().get("showStatusBar",true)) bar.show(); else bar.hide();
  }
  async function refreshCodex() {
    if(reloading) return; reloading = true;
    try {
      // These are VS Code workbench commands, not OpenAI APIs. Probe support first.
      const commands = await vscode.commands.getCommands(true);
      if(commands.includes("workbench.action.restartExtensionHost")) {
        try { await vscode.commands.executeCommand("workbench.action.restartExtensionHost"); return; } catch { debug("REFRESH_FAILED"); }
      }
      await vscode.commands.executeCommand("workbench.action.reloadWindow");
    } catch { debug("REFRESH_FAILED"); void vscode.window.showWarningMessage("Codex profile changed. Reload this VS Code window to apply it."); }
    finally { reloading = false; }
  }
  // Serialize metadata persistence before restarting to prevent reconnect/reload loops.
  let events = Promise.resolve();
  async function eventReceived(event: ProfileEvent) {
    if(event.homeKey !== key) return; // Isolated windows must never follow global switching.
    profile = event.profileName ?? ""; render();
    const previous = context.globalState.get<string | null>(stateKey);
    await context.globalState.update(stateKey, event.profileId);
    if(event.profileId && previous !== event.profileId && (previous !== undefined || event.type === "PROFILE_CHANGED")) {
      if(config().get("autoReloadCodex",true)) await refreshCodex();
      else void vscode.window.showInformationMessage("Codex account changed. Reload when your current task is complete.","Reload").then(action => { if(action) void refreshCodex(); });
    }
  }
  function connect() {
    if(disposed || blocked || (socket && !socket.destroyed) || process.platform !== "win32" || vscode.env.remoteName) return;
    const decoder = new LineDecoder(); socket = net.createConnection(pipe); socket.setEncoding("utf8");
    socket.on("connect", () => { connected = true; reconnectDelay = 500; debug("CONNECTED"); render(); });
    socket.on("data", (chunk: string) => { try { for(const event of decoder.push(chunk)) events = events.then(() => eventReceived(event)).catch(() => debug("INVALID_FRAME")); } catch { debug("INVALID_FRAME"); socket?.destroy(); } });
    socket.on("error", () => {}); // Never log OS messages or incoming data.
    socket.on("close", () => { connected = false; render(); debug("DISCONNECTED"); if(!disposed && !blocked) { retry = setTimeout(connect,reconnectDelay); reconnectDelay = Math.min(reconnectDelay * 2,15000); } });
  }
  async function conflicts() {
    const other = vscode.extensions.getExtension("woozy-masta.codex-switch");
    if(!other) return;
    blocked = true; socket?.destroy(); if(retry) clearTimeout(retry); render();
    const result = await vscode.window.showWarningMessage("Another Codex account manager is enabled. Running multiple switchers can cause authentication conflicts.", {modal:true}, "Open Extensions to Disable", "Continue Anyway");
    if(result === "Open Extensions to Disable") { await vscode.commands.executeCommand("workbench.extensions.search","@id:woozy-masta.codex-switch"); }
    else if(result === "Continue Anyway") { blocked = false; connect(); }
  }
  context.subscriptions.push(output,bar,
    vscode.commands.registerCommand("codexAccountManager.refreshCodex",refreshCodex),
    vscode.commands.registerCommand("codexAccountManager.showStatus", () => vscode.window.showInformationMessage(connected ? `Codex Account Manager connected. Profile: ${profile || "No matching profile"}` : "Open the Codex Account Manager desktop app. This companion supports local Windows VS Code windows.")),
    vscode.workspace.onDidChangeConfiguration(render),
    vscode.extensions.onDidChange(() => { if(vscode.extensions.getExtension("woozy-masta.codex-switch")) void conflicts(); else if(blocked) { blocked = false; connect(); } }),
    {dispose() { disposed = true; if(retry) clearTimeout(retry); socket?.destroy(); }});
  render();
  void conflicts().then(() => { if(!blocked) connect(); });
}
