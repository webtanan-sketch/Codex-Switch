You are a senior Windows desktop engineer, Rust engineer, TypeScript engineer, VS Code extension engineer, security engineer, and software architect.

Build a production-ready Windows application named:

# Codex Account Manager

The application is a local-only account manager for OpenAI Codex on Windows.

The primary goal is to allow a user to save and manage multiple Codex / ChatGPT authenticated accounts and instantly switch the account used by the Codex VS Code extension without repeatedly logging out and logging back in.

The project must be designed for Windows 10/11 x64.

Do not create a prototype.
Do not create mock-only functionality.
Implement the real switching architecture.

---

# 1. Important Codex Authentication Model

Codex CLI and the Codex VS Code extension can share the same authentication state.

The default Codex home is:

%USERPROFILE%\.codex

The default file-based authentication location is:

%USERPROFILE%\.codex\auth.json

When CODEX_HOME is defined, the authentication path becomes:

<CODEX_HOME>\auth.json

Do NOT assume that the auth.json schema will always remain identical.

Treat auth.json as an opaque authentication payload whenever possible.

Preserve unknown fields.

Never reconstruct access tokens manually.

Never decode, modify, manufacture, or regenerate OAuth tokens.

Raw authentication tokens must never appear in application logs.

---

# 2. Application Architecture

Create a monorepo:

codex-account-manager/
    apps/
        desktop/
        vscode-extension/

    crates/
        account-core/
        codex-auth/
        profile-vault/
        vscode-ipc/

    packages/
        shared-types/

    installer/

    tests/

Desktop stack:

Tauri 2
Rust
React
TypeScript
Vite

VS Code extension:

TypeScript
Official VS Code Extension API

Do not use Electron unless absolutely required.

---

# 3. Two Operating Modes

The application must support two different modes.

## Mode A — Global Account Switching

This changes the account used by the normal default Codex installation.

Active authentication path:

%USERPROFILE%\.codex\auth.json

Example accounts:

Webtanan
PlayFood
Goharfaam
Personal

When the user selects:

Switch → PlayFood

the application must:

1. acquire an inter-process switch lock
2. read the current auth.json
3. validate that it is valid JSON
4. identify the currently active profile
5. synchronize the latest current auth.json into the encrypted vault for the currently active profile
6. decrypt the selected target profile
7. validate its JSON structure without modifying token fields
8. write it to auth.json.tmp
9. flush the completed file
10. atomically replace auth.json
11. read auth.json again
12. verify that the target profile identity is active
13. update the application active-profile state
14. notify all connected VS Code companion instances
15. instruct the VS Code companion to refresh Codex
16. release the lock

Never overwrite auth.json in-place.

Use temporary-file + atomic rename/replace semantics.

A crash during switching must leave either the old valid file or the new valid file, never a partially written auth.json.

---

# 4. Mode B — Isolated VS Code Accounts

Allow multiple Codex accounts to be open at the same time in separate VS Code processes.

Create profile-specific Codex homes:

%USERPROFILE%\.codex-hub\homes\webtanan
%USERPROFILE%\.codex-hub\homes\playfood
%USERPROFILE%\.codex-hub\homes\goharfaam

Each profile home can contain:

auth.json
config.toml
sessions/
logs/
skills/

Add a button:

Open New VS Code

The desktop app must start VS Code as a genuinely separate process with:

CODEX_HOME=<profile specific home>

If required to guarantee process isolation, use a profile-specific VS Code user-data directory:

%USERPROFILE%\.codex-hub\vscode-data\<profile-id>

The user must be able to run:

VS Code Window A → Webtanan account
VS Code Window B → PlayFood account
VS Code Window C → Goharfaam account

at the same time.

Never attempt to change CODEX_HOME of an already-running process.

CODEX_HOME is process startup state.

---

# 5. Secure Credential Vault

Authentication data is sensitive.

Do NOT store saved accounts as unencrypted auth.json copies.

Use Windows DPAPI or another Windows per-user protected credential mechanism.

Preferred implementation:

Windows DPAPI
CryptProtectData
CryptUnprotectData

Store only encrypted binary credential blobs on disk.

Suggested app data location:

%APPDATA%\CodexAccountManager

Example:

CodexAccountManager/
    profiles.db
    state.json
    settings.json

    vault/
        <profile-id>.bin

    logs/
        app.log

Metadata may contain:

profile id
profile display name
email if safely extracted
account identity hash
plan label if available
created time
last used time
last synchronized time

Metadata must NEVER contain:

access_token
refresh_token
id_token
raw auth.json
API keys

---

# 6. Authentication Identity Matching

Do not identify profiles by comparing entire auth.json files.

Tokens can refresh.

Determine identity using stable identity information when available.

Potential identity fields may include:

JWT sub
userId
chatgptUserId
accountId
workspace / organization identifiers
email

Create an identity fingerprint.

Prefer stable user identifiers over email.

Do not log the raw JWT.

If JWT parsing is needed only to read non-secret claims, parse locally and never print the token.

---

# 7. Token Refresh Synchronization

Codex may refresh authentication tokens while it is running.

This means the currently active auth.json can become newer than the saved profile.

Implement an auth file watcher.

Watch:

<active CODEX_HOME>\auth.json

When the file changes:

1. debounce the event
2. wait until the file is stable
3. parse it
4. determine its identity
5. confirm that identity matches the active profile
6. encrypt the updated auth payload using DPAPI
7. update the saved profile vault
8. update lastSyncedAt

Do NOT trigger a VS Code restart when Codex itself merely refreshes a token.

Distinguish:

TOKEN_REFRESH

from:

PROFILE_SWITCH

Only PROFILE_SWITCH should cause VS Code Codex reload behavior.

---

# 8. VS Code Companion Extension

Create a separate companion extension.

Suggested extension ID:

webtanan.codex-account-manager

Responsibilities:

- detect whether Codex Account Manager is available
- connect only through local IPC
- receive profile-change events
- display current Codex profile in VS Code status bar
- refresh Codex after a profile switch
- never store account credentials
- never transmit auth.json
- never transmit OAuth tokens

When PROFILE_CHANGED is received:

First attempt to restart the VS Code extension host using the appropriate VS Code command.

If extension host restart is not supported or fails, fall back to:

workbench.action.reloadWindow

The user should not normally need to manually restart VS Code.

Add settings:

codexAccountManager.autoReloadCodex
codexAccountManager.showStatusBar
codexAccountManager.debugLogging

Default:

autoReloadCodex = true
showStatusBar = true
debugLogging = false

---

# 9. Desktop ↔ VS Code Communication

Communication must be local-only.

Preferred Windows transport:

Windows Named Pipe

Suggested pipe:

\\.\pipe\CodexAccountManager

The desktop process should be the server.

All active VS Code companion extensions can connect as clients.

The desktop app must broadcast profile switch events.

Example message:

{
  "type": "PROFILE_CHANGED",
  "profileId": "playfood",
  "profileName": "PlayFood",
  "timestamp": 0
}

Do not send authentication data over IPC.

Messages must only contain non-secret profile information and control events.

Reconnect automatically if the desktop process restarts.

---

# 10. Account Management UI

Create a modern minimal desktop interface.

Main screen:

Codex Account Manager

Accounts

[ + Add Account ]

Account cards:

Webtanan
email
ChatGPT plan if known
Active

[ Open VS Code ]

PlayFood
email
ChatGPT plan if known

[ Switch ]
[ Open VS Code ]

Goharfaam
email

[ Switch ]
[ Open VS Code ]

Active account must have a clear green indicator.

Do not display access tokens.

Do not expose raw account IDs unless the user explicitly opens technical details.

---

# 11. Add Account Wizard

Implement an Add Account flow.

Steps:

1. Preserve current active profile.
2. Prepare a new Codex authentication flow.
3. Allow the user to authenticate through Codex / ChatGPT.
4. Detect newly generated authentication state.
5. Read the resulting auth payload.
6. Extract safe profile metadata.
7. Ask user for a friendly profile name.
8. Encrypt auth payload with DPAPI.
9. Store profile.
10. Optionally restore the previously active account.

Possible UI:

Add Codex Account

Profile name:
[                         ]

[ Login with ChatGPT ]

Status:
Waiting for Codex authentication...

When detected:

Account detected successfully.

[ Save Account ]

Do not use Logout as the normal account registration mechanism if it invalidates existing sessions.

Design the flow to preserve the previous credential before preparing the next login.

---

# 12. Existing Codex Switch Detection

At startup inspect:

%USERPROFILE%\.codex-switch

A directory such as:

.codex-switch\maintenance\v1\...

must NOT be interpreted as credential storage.

Treat maintenance/v1 as cache / coordination data.

If these exist:

.codex-switch\profiles.json
.codex-switch\profiles\

offer:

Import Existing Codex Switch Profiles

Only import data when its format has been positively recognized.

Do not scrape arbitrary JSON files looking for OAuth tokens.

If existing credentials are stored in VS Code SecretStorage and cannot safely be accessed, explain that automatic migration is unavailable and allow the user to import the currently active auth.json one account at a time.

---

# 13. Conflict Detection

Detect known Codex switching extensions if possible.

In particular detect:

woozy-masta.codex-switch

If another account switcher capable of writing Codex auth state is enabled, show:

Another Codex account manager is active.

Running multiple Codex account switchers can cause authentication conflicts.

Options:

Disable conflicting extension
Continue anyway
Cancel

Never silently disable another extension.

---

# 14. Credential Storage Mode Detection

Inspect the Codex configuration.

Codex may use:

file
keyring
auto
ephemeral

For MVP, reliable account switching should use file-based authentication.

If the current installation is using OS keyring authentication instead of auth.json, detect it.

Show a clear setup screen rather than silently changing settings.

Example:

Codex is currently using secure OS credential storage.

Codex Account Manager requires file-based authentication for profile switching.

[ Configure File-Based Authentication ]

If the user chooses this option, carefully update:

%USERPROFILE%\.codex\config.toml

with:

cli_auth_credentials_store = "file"

Preserve all other config.toml values and formatting when practical.

Create a backup before modifying config.toml.

Never destroy existing MCP, skills, model, approval, sandbox, or other Codex settings.

---

# 15. Windows System Tray

Implement a system tray menu.

Example:

Codex Hub

● Webtanan
○ PlayFood
○ Goharfaam
○ Personal

----------------

Open Webtanan VS Code
Open PlayFood VS Code

----------------

Open Manager
Settings
Exit

Selecting an account from the tray should perform the same safe switch transaction used by the main UI.

Do not implement a separate switching path.

All switching must go through the same core service.

---

# 16. Logs

Logs may contain:

time
operation
profile display name
success / failure
error category
file path
VS Code connection status

Logs must never contain:

access tokens
refresh tokens
ID tokens
auth.json contents
Authorization headers
API keys

Implement a centralized redaction layer.

Any string resembling JWT or OAuth credentials should be redacted before logging.

---

# 17. Backups and Recovery

Before destructive operations, preserve recoverability.

Provide:

Settings → Recovery

Actions:

Restore previous active account
Repair active auth file
Re-import active Codex account
Reset application state without deleting Codex authentication

If a profile switch fails after the target file has been prepared but before verification succeeds:

rollback to previous valid authentication.

Never leave the user logged out as the result of a failed profile switch if a valid previous credential is available.

---

# 18. File Locking

Multiple instances of the application must not switch authentication simultaneously.

Implement:

global application mutex

and

auth switch mutex.

Example Windows mutex:

Global\CodexAccountManagerSwitch

If another switch is in progress, queue or reject the second operation cleanly.

Also account for Codex itself writing auth.json during token refresh.

---

# 19. First Run Diagnostics

On first run detect:

Windows version
User profile
VS Code installation path
Codex CLI availability
Codex VS Code extension availability
CODEX_HOME
auth.json presence
config.toml presence
credential storage mode
existing .codex-switch installation

Display:

System Check

✓ VS Code
✓ Codex Extension
✓ Codex auth.json
✓ File authentication
✓ VS Code Companion
✓ Local IPC

Do not expose tokens in diagnostic output.

---

# 20. VS Code Discovery

Support common VS Code installations:

User installation
System installation

Possible executable:

%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe

Also attempt:

where code

Do not hardcode only one VS Code location.

Allow manual VS Code executable selection.

Persist the selected executable path.

---

# 21. UI Language

Primary UI language should be Persian.

English technical terms may appear alongside Persian labels.

Examples:

حساب فعال
Active Account

تعویض حساب
Switch Account

باز کردن VS Code
Open VS Code

افزودن حساب
Add Account

وضعیت اتصال
Connection Status

تنظیمات
Settings

بازیابی
Recovery

Use proper RTL layout for Persian UI.

Technical file paths and code values must remain LTR.

---

# 22. Testing

Create automated tests for at least:

profile encryption/decryption
identity matching
atomic auth replacement
rollback
auth file watcher
token refresh synchronization
switch locking
invalid auth.json handling
missing auth.json
corrupted target profile
unknown auth fields preservation
VS Code event serialization
secret redaction
profile migration
CODEX_HOME resolution

Create failure injection tests.

Example:

Crash after temp file write but before rename.

Expected:

original auth.json remains valid.

Example:

Crash immediately after rename.

Expected:

new auth.json is complete and valid.

---

# 23. Acceptance Tests

The implementation is not complete until these scenarios work.

TEST 1

User saves Account A and Account B.

Account A is active.

User clicks:

Switch → Account B

Result:

auth.json belongs to B.

VS Code automatically refreshes.

Codex UI opens using B.

No browser login is required.

---

TEST 2

Codex refreshes Account B credentials.

Application detects changed auth.json.

Updated B authentication is encrypted into B's profile vault.

Later switching A → B still works.

---

TEST 3

User launches:

Open VS Code → Account A

and:

Open VS Code → Account B

Both windows remain open simultaneously with independent CODEX_HOME values.

---

TEST 4

Application crashes during account switching.

Codex auth.json is never partially written.

On next launch Recovery identifies the state.

---

TEST 5

Debug logging enabled.

Search every log file.

There must be zero raw access tokens, refresh tokens, ID tokens, or raw auth.json payloads.

---

# 24. Security Rules

Absolute requirements:

Never upload credentials.

Never use a cloud backend.

Never send auth.json to telemetry.

Never send account credentials to localhost HTTP services.

Never log credentials.

Never commit credentials.

Never create sample auth.json files containing realistic tokens.

Never modify token payloads.

Never attempt to generate OAuth refresh tokens.

Never silently log out a Codex account.

Never delete the only known valid authentication state without first preserving a recoverable copy.

---

# 25. Development Workflow

Work in phases.

PHASE 0
Inspect environment and existing project.

PHASE 1
Create architecture and technical design.

PHASE 2
Implement Rust authentication/profile core.

PHASE 3
Implement DPAPI credential vault.

PHASE 4
Implement atomic switching engine.

PHASE 5
Implement token refresh file watcher.

PHASE 6
Implement Tauri desktop UI.

PHASE 7
Implement VS Code Companion.

PHASE 8
Implement Named Pipe integration.

PHASE 9
Implement isolated CODEX_HOME VS Code launcher.

PHASE 10
Implement migration/conflict detection.

PHASE 11
Testing and security audit.

PHASE 12
Build Windows installer.

Do not skip tests in order to finish faster.

After each phase:

run tests
fix TypeScript errors
fix Rust warnings where practical
verify no credentials are present in repository files
commit logically separated changes

---

# 26. Deliverables

Final deliverables must include:

Windows installer

Codex Account Manager desktop app

VS Code companion .vsix

source code

README-FA.md

README.md

SECURITY.md

architecture documentation

recovery documentation

test suite

build scripts

release scripts

The application must be usable by a non-technical Windows user.

Final user experience:

1. Install Codex Account Manager.
2. Install VS Code Companion.
3. Add ChatGPT/Codex accounts once.
4. Name each account.
5. Click an account.
6. Codex in VS Code changes to that account automatically.
7. No repeated logout/login.
8. Optionally open several VS Code windows with different accounts simultaneously.

Before implementing any authentication mutation logic, inspect the currently installed Codex behavior and current auth/config files without printing secrets.

If an assumption about the current Codex implementation cannot be verified, do not guess. Isolate that behavior behind an adapter and document the uncertainty.
