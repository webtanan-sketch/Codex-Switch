# Codex Account Manager — VS Code Companion

A local Windows companion for [Codex Account Manager](https://github.com/webtanan-sketch/Codex-Switch). Install the desktop application separately.

The companion receives **profile metadata only** through a user-restricted Windows named pipe. It never reads, writes, stores, or transmits Codex credentials. After a global account change, it restarts the extension host; if that command is unavailable or fails, it reloads the window. Save work before switching accounts. Disable `codexAccountManager.autoReloadCodex` to refresh manually.

## Setup

1. Install the desktop application and start it.
2. In VS Code, run **Extensions: Install from VSIX…** and select the built companion `.vsix`.
3. Import your active Codex account in the desktop manager.
4. Switch profiles in the manager. The status bar shows the selected profile.

For isolated windows, install both this companion and the official Codex extension in each window's separate VS Code user-data environment. Global switch events deliberately do not reload isolated windows.

## Settings

| Setting | Default | Purpose |
| --- | --- | --- |
| `codexAccountManager.autoReloadCodex` | `true` | Apply account changes automatically |
| `codexAccountManager.showStatusBar` | `true` | Show local connection and profile |
| `codexAccountManager.debugLogging` | `false` | Log connection categories only |

Use **Codex Account Manager: Refresh Codex** for manual refresh. Remote SSH, WSL, and container extension hosts are not supported. Existing `woozy-masta.codex-switch` installations trigger a conflict prompt; another extension is never disabled automatically.

This is an independent community project, not an OpenAI product. The `webtanan` Marketplace publisher namespace is a suggested identity and has not been verified or published by this repository.
