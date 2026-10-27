<div align="center">

# Codex Account Manager

**Your Codex accounts. One local workspace.**

A Windows desktop account manager with a VS Code companion. Save your ChatGPT sign-ins once, switch the account used by your local Codex installation, or open separate VS Code environments for different accounts.

[فارسی](README-FA.md) · [Architecture](docs/ARCHITECTURE.md) · [Recovery](docs/RECOVERY.md) · [Security](SECURITY.md) · [Contributing](CONTRIBUTING.md)

</div>

**[Project page](https://webtanan-sketch.github.io/Codex-Switch/) · [Download Windows installer](https://github.com/webtanan-sketch/Codex-Switch/releases/download/v0.1.2/codex-account-manager-0.1.2-x64-setup.exe) · [Standalone EXE](https://github.com/webtanan-sketch/Codex-Switch/releases/download/v0.1.2/codex-account-manager-0.1.2.exe) · [Release details and SHA-256](https://github.com/webtanan-sketch/Codex-Switch/releases/tag/v0.1.2)**

> **Windows preview — v0.1.2.** The installer and standalone executable are unsigned. Real switching between two accounts has not been fully validated; review the [validation status](docs/VALIDATION.md) and the release notes before use.
>
> An independent community project by Webtanan. Not affiliated with or endorsed by OpenAI or Microsoft.

## What it does

- **Global account switching:** change the file-backed login used by Codex CLI and the local VS Code Codex extension.
- **Independent VS Code windows:** give each saved account a separate `CODEX_HOME` and VS Code user-data directory.
- **Encrypted saved credentials:** Windows DPAPI protects vault blobs for the current Windows user.
- **Automatic credential synchronization:** stable-file polling saves refreshed authentication without restarting VS Code.
- **Automatic VS Code refresh:** a companion extension receives local profile-change metadata and restarts the extension host, with window reload as a fallback.
- **Recoverable switching:** preserve the previous login, write and flush a temporary file, replace atomically, verify, and commit metadata. A journal supports recovery after interruption.
- **Persian desktop UI:** RTL layout, Persian calendar dates, English technical labels, account cards, system checks, recovery controls, and a system tray.
- **Local communication:** an owner-restricted Windows named pipe; no HTTP listener, cloud backend, analytics, or credential telemetry.

## Requirements

| Component | Requirement |
| --- | --- |
| Operating system | Windows 10 or Windows 11, x64 |
| Desktop runtime | Microsoft Edge WebView2 Runtime; the NSIS bootstrapper can install it when needed |
| Editor | Local Microsoft Visual Studio Code with the official OpenAI Codex extension |
| Authentication | A supported ChatGPT/Codex login and explicit file-backed credential storage |
| Login tool | Codex CLI native `codex.exe`, available on `PATH`, found under a standard npm installation, or selected in Settings |
| Companion | Install the `.vsix` built from this repository |

The manager does not provide Codex access, a subscription, additional quota, or permission to use another person's account. Normal Codex login and service connectivity are still required. API-key, workload-federation, remote SSH, WSL, and dev-container profiles are outside this preview's scope.

## Install and start

### Preview build v0.1.2

Download the [Windows x64 installer](https://github.com/webtanan-sketch/Codex-Switch/releases/download/v0.1.2/codex-account-manager-0.1.2-x64-setup.exe) or the [standalone EXE](https://github.com/webtanan-sketch/Codex-Switch/releases/download/v0.1.2/codex-account-manager-0.1.2.exe) from the [v0.1.2 release page](https://github.com/webtanan-sketch/Codex-Switch/releases/tag/v0.1.2). Verify the SHA-256 checksum listed there before running an unsigned preview build.

1. Run the Windows NSIS installer (`*-setup.exe`). Unsigned development builds may trigger Windows SmartScreen; verify the source and checksum before installing.
2. Start **Codex Account Manager** as your normal Windows user. Administrator access is not needed.
3. In VS Code, open the Command Palette and run **Extensions: Install from VSIX…**. Select the companion `.vsix` packaged with the source or build it using the instructions below.
4. Install the official OpenAI Codex extension if it is not already installed.
5. Open **Settings / تنظیمات** in the manager and review **System Check**.

### First-time setup

1. If Codex uses `keyring`, `auto`, `ephemeral`, or an unspecified store, choose **Configure File-Based Authentication** in the manager. This is an explicit action that backs up `config.toml` before updating the storage setting.
2. Complete a Codex login using that storage mode if there is no `auth.json`. Changing the setting does **not** export a keyring credential.
3. Select **Add account / افزودن حساب**, enter a friendly name, and choose **Import current account**.
4. Add another account with **Login with ChatGPT**. Complete the official Codex login in the browser, then choose **Save account**.
5. Return to Accounts and click **Switch account / تعویض حساب**.

The Add Account flow uses a fresh temporary Codex home, so it does not log out or overwrite the current global account. The new account becomes globally active only when you select Switch.

Organization-managed authentication requirements can override user configuration. The manager does not bypass those policies. If a managed installation continues using another storage backend, global switching is not supported; confirm the resulting behavior with your administrator.

## Two ways to use your accounts

### Global switching

A global switch updates `<CODEX_HOME>\auth.json`. With no override, the home is `%USERPROFILE%\.codex`.

The manager preserves the latest registered active login, decrypts the selected profile locally, replaces the auth file, verifies the selected identity, and notifies the companion. Saved payload bytes are preserved exactly, including unknown JSON fields.

The companion attempts `workbench.action.restartExtensionHost`. If unavailable or unsuccessful, it calls `workbench.action.reloadWindow`. Save ongoing work first: restarting the extension host affects other extensions as well. Disable automatic refresh if you need to finish a task before changing accounts in the editor.

Windows that use a different Codex home ignore global profile notifications. Changing a process environment variable cannot update an already-running VS Code process; start a new process after changing `CODEX_HOME`.

### Isolated windows

Click **Open VS Code / باز کردن VS Code** on an account card. The manager launches `Code.exe` with:

```text
CODEX_HOME=%USERPROFILE%\.codex-hub\homes\<profile-id>
--new-window --user-data-dir %USERPROFILE%\.codex-hub\vscode-data\<profile-id>
```

Different user-data directories force separate VS Code environments. The manager now provisions official Codex automatically in an explicit per-account extensions directory before opening the window. First setup requires Marketplace access. Install the companion VSIX into that same environment. See the [offline setup and missing Codex guide](docs/BUILD-WINDOWS.md#codex-missing-from-an-accounts-vs-code-window). Editor settings, sessions, extensions, and Codex configuration are independent; the manager does not automatically copy your global MCP, skills, or project settings. Existing per-profile configuration is preserved when its credential-store setting is updated.

A global switch does not change these isolated windows. The companion's global profile label may remain unset in an isolated window; that window's own Codex session is authoritative. Avoid using the **same** saved account concurrently in global and isolated processes until token-rotation behavior has been validated for your Codex version. Independent account windows are the intended use case.

## Usage limits and switching setup

The active account now shows the remaining quota percentage, the window length reported by Codex, and the next reset date/countdown. A five-hour or weekly window is shown only when Codex actually reports it. Missing quota data is shown as unavailable, never as unused quota. The reset countdown is informational; it does not promise that usage is permitted after that time.

Quota metadata is read through the configured native `codex.exe` using the official [app-server account methods](https://developers.openai.com/codex/app-server): `account/read` with `refreshToken: false`, then `account/rateLimits/read`. The manager does not start an AI turn, log in/out, redeem reset credits, or activate an inactive saved account to inspect it. Normal credential refresh, if needed, remains the official Codex process's responsibility. Reads are cached for one minute; the panel refreshes automatically every five minutes. Account changes and late responses are checked before applying a result.

If file-backed storage is not configured, **Switch account** opens an explicit setup confirmation with the target account name. Confirming backs up `config.toml`, enables file storage through the existing configuration adapter, then switches. Starting the manager or reading quota does not change this setting.

Two optional local diagnostics are available after the usual Windows build prerequisites:

```powershell
cargo run -p account-core --example usage_check --locked
# Opens a dedicated, empty VS Code test environment, without selecting a saved account:
cargo run -p account-core --example launch_check --locked
```

`usage_check` prints quota metadata and identity/configuration preservation booleans only. Never attach real credentials to test reports.

## Where data lives

| Location | Contents |
| --- | --- |
| `%APPDATA%\CodexAccountManager\state.json` | Profile names, identity fingerprints, optional email/plan, timestamps, active/previous profile pointers |
| `%APPDATA%\CodexAccountManager\vault\<id>.bin` | DPAPI-encrypted opaque auth payload |
| `%APPDATA%\CodexAccountManager\rollback.bin` | One DPAPI-encrypted recovery payload |
| `%APPDATA%\CodexAccountManager\transaction.json` | Non-secret journal for an interrupted switch |
| `%APPDATA%\CodexAccountManager\settings.json` | Explicit executable paths |
| `%APPDATA%\CodexAccountManager\logins\<id>` | Temporary, access-restricted Codex login home; removed after successful encrypted save |
| `%USERPROFILE%\.codex-hub\homes\<id>` | Per-profile runtime Codex home |
| `%USERPROFILE%\.codex-hub\vscode-data\<id>` | Independent VS Code environment |
| `<CODEX_HOME>\auth.json` | The active, file-backed Codex login |

**Saved vault blobs are encrypted. Active runtime auth files are plaintext because Codex requires them.** Runtime and staging directories are access restricted, but malware running as your Windows user can still access your sessions and invoke DPAPI. DPAPI backups are not portable account exports and may become unusable after changing Windows accounts or reinstalling Windows. See the [security model](SECURITY.md).

## Companion settings

In VS Code Settings, search for **Codex Account Manager**.

| Setting | Default | Effect |
| --- | --- | --- |
| `codexAccountManager.autoReloadCodex` | `true` | Automatically refresh after an account change |
| `codexAccountManager.showStatusBar` | `true` | Show manager connection and matching global profile |
| `codexAccountManager.debugLogging` | `false` | Log connection categories only |

The companion reconnects automatically when the manager restarts. It records the last applied profile ID, preventing a restart loop when reconnecting. It does not read `auth.json`, receive OAuth tokens, or store account credentials.

## Troubleshooting

| Symptom | Action |
| --- | --- |
| Codex does not change accounts | Confirm file storage, matching `CODEX_HOME`, a running manager, installed companion, and automatic reload enabled. Review organization policies. |
| Codex CLI not found | Install Codex CLI; select the native `codex.exe` in Settings. Shell scripts and `.cmd` wrappers are intentionally not executed. |
| VS Code not found | Select the absolute `Code.exe` path in Settings; user and system installs are searched automatically. |
| No file after login | Finish the browser flow and ensure your Codex installation allows file storage. Then choose Save account again. |
| Another account manager is detected | Disable it through VS Code's Extensions UI, or explicitly choose Continue Anyway. The companion never disables another extension silently. |
| Unknown active account | Import the currently active login before switching, so it can be preserved. |
| Unsupported authentication identity | The adapter requires a stable user identity, with workspace identity when present. Report the Codex version and error category; **never attach credentials**. |
| Interrupted switch or damaged file | Use the Recovery page and follow [Recovery](docs/RECOVERY.md). |
| A second instance cannot switch | A filesystem inter-process lock serializes account operations; wait for the current operation to finish. |
| Window disappeared when closed | The desktop manager stays in the system tray. Choose Open Manager or Exit there. |
| Old Codex Switch profiles found | Use the old switcher to activate each account, then import the current login here. Arbitrary JSON/SecretStorage migration is not supported. |

`~/.codex-switch/maintenance/v1/.../profiles` is coordination/cache data. It is not treated as an account vault, and no arbitrary files are scanned for OAuth credentials.

## Build from source

### Windows prerequisites

- Git, Node.js 22 or newer, and npm.
- Current stable Rust through [rustup](https://rustup.rs/), including the MSVC toolchain.
- Visual Studio 2022 Build Tools with **Desktop development with C++** and a Windows SDK.
- WebView2 Runtime. See [Tauri's Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows).

Run in PowerShell:

```powershell
git clone https://github.com/webtanan-sketch/Codex-Switch.git
cd Codex-Switch
npm ci
cargo test --workspace --exclude codex-account-manager
npm run typecheck
npm test
npm run desktop:dev
```

Create the desktop installer and companion:

```powershell
npm run desktop:build
npm run extension:package
```

Outputs:

```text
target\release\bundle\nsis\*-setup.exe
apps\vscode-extension\codex-account-manager-0.1.0.vsix
```

Use `installer/build.ps1` to run validation and build both artifacts. Production vault operations fail closed on non-Windows platforms. Pure core tests and frontend/extension builds can run on Linux, but they do not replace Windows DPAPI, named-pipe, installer, or real Codex acceptance testing.

### Repository layout

```text
apps/
  desktop/              React + TypeScript + Vite + Tauri 2 desktop
  vscode-extension/     Credential-free VS Code companion
crates/
  account-core/         Transactions, recovery, watcher, process adapters
  codex-auth/           Opaque payload validation and stable identity hints
  profile-vault/        DPAPI, restricted directories, atomic writes
  vscode-ipc/           Metadata protocol and Windows named-pipe server
packages/shared-types/  Public frontend metadata contracts
docs/                   Architecture, recovery, validation, development plan
installer/              Windows build entry point
scripts/                Source credential scan and release helpers
tests/                  Manual real-account acceptance protocol
```

## Testing and release policy

The test suite covers identity changes, refreshed payloads, unknown-field preservation, atomic replacement, injected interruptions before/after replacement, verification rollback, inter-process locking, invalid/missing auth files, corrupt vaults, isolated homes, login staging, configuration backups, and protocol validation.

Windows tests use real DPAPI. Non-Windows transaction tests inject a **test-only** reversible protector; the shipping application cannot select it. A Windows CI build produces an **unsigned preview** installer and VSIX as Actions artifacts. Release packaging creates a **draft**, never a public release automatically. [Manual real-account acceptance tests](tests/ACCEPTANCE.md) and signing review are required before general distribution.

No workflow requires a ChatGPT token, API key, or personal `auth.json`. Never upload one as a CI secret or artifact for this project's tests.

## Current boundaries

- No Linux/macOS desktop, browser login implementation of our own, keyring extraction, credential export, or automatic legacy-vault migration.
- Account removal, rename, installer upgrade/uninstall preservation checks, and dedicated per-profile companion status remain follow-up work.
- Background synchronization uses periodic stable reads, not native filesystem event notifications.
- A file lock coordinates manager instances; Codex itself does not participate. Windows write-sharing restrictions and pre/post checks reduce races but cannot guarantee coordination with every independent external writer.
- Identity claims are parsed locally as **unverified matching hints**, never used as proof of authorization. Future unsupported auth formats are rejected rather than guessed.
- Workbench refresh commands and Codex's cached-login behavior are version-sensitive integration adapters, not a guaranteed OpenAI account-switching API.
- The repository has no code-signing certificate or verified Marketplace publisher configuration.

See [Roadmap](docs/ROADMAP.md) for remaining delivery gates.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md). Report bugs with OS, Codex, VS Code, and manager versions, reproduction steps, and sanitized error categories. Do not share auth files, encrypted vault blobs, temporary login directories, or private account identifiers.

## Official integration references

- [Codex authentication and credential storage](https://developers.openai.com/codex/auth)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)
- [VS Code process isolation and CLI](https://code.visualstudio.com/docs/configure/command-line#_isolating-vs-code-instances)
- [VS Code command API](https://code.visualstudio.com/api/references/vscode-api#commands)
- [Windows DPAPI: CryptProtectData](https://learn.microsoft.com/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)

## License

[Apache License 2.0](LICENSE). OpenAI, ChatGPT, Codex, Visual Studio Code, and related names belong to their respective owners.

## Local EXE build / ساخت محلی

[Build on Windows without GitHub Actions, including ZIP source](docs/BUILD-WINDOWS.md).

The desktop language selector supports Persian (RTL, Shamsi dates) and English (LTR, Gregorian dates). Your selection is saved locally.
