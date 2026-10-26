# Architecture

## Trust boundaries

The Tauri Rust process owns authentication operations. React receives safe profile metadata only. The VS Code extension is a one-way named-pipe client: it cannot switch accounts, request credentials, or access a vault through the protocol. No network service or cloud backend is part of the manager.

`codex-auth` accepts opaque JSON bytes and extracts stable user/workspace identity hints. Unknown fields and original formatting survive import and switching. API-key-only payloads or payloads without supported identity hints fail closed. JWT claim decoding is local and unverified; it is used for matching only, not authentication or authorization.

`profile-vault` wraps current-user Windows DPAPI without machine-wide protection. Vault and managed runtime directories receive protected DACLs for the current user and SYSTEM. Non-Windows production protection returns Unsupported. Sensitive byte buffers use zeroization; parsed JSON strings, allocator copies, crash dumps, swap, and Codex-owned memory are not fully zeroized.

`account-core` owns the profile index, switching, explicit setup, isolated login staging, recovery, stable-file polling, executable discovery, and process launch. UI and tray both call the same switch service. Tauri's single-instance plugin prevents duplicate GUI instances; an OS-backed file lock serializes all core mutations across processes. A named global mutex was intentionally replaced by a per-user, filesystem-backed inter-process lock to avoid cross-user global-namespace privileges and collisions.

`vscode-ipc` exposes a metadata-only versioned protocol over `\\.\pipe\CodexAccountManager`. Its DACL uses current-user/SYSTEM permissions and rejects remote clients. The server reserves the first pipe instance to fail when the name is already occupied. Same-user malicious code is outside the confidentiality boundary and could impersonate a manager while it is absent. The companion therefore treats notifications as untrusted metadata and never accepts tokens or executable commands.

## Switch transaction

1. Acquire the exclusive core operation lock and reject any unresolved journal.
2. Require explicitly configured file storage.
3. Decrypt and validate the target; verify its identity against the profile index.
4. Read and validate existing active auth. Refuse to overwrite an unregistered valid account.
5. On Windows, hold the old auth file open with write-sharing denied and delete-sharing allowed. Verify it has not changed.
6. Encrypt the latest active payload into its own vault entry and persist metadata.
7. If selecting the same account, keep the latest auth bytes and return without replacing them.
8. Save the old login to `rollback.bin`, then durably write the non-secret transaction journal.
9. Recheck the active snapshot, write the target into a sibling temporary file, flush, and atomically replace `auth.json` using tempfile's platform implementation.
10. Re-read and verify target identity and byte digest. Restore the encrypted old payload on verification failure.
11. Persist active and previous pointers, remove the journal, and broadcast `PROFILE_CHANGED`.

Before replacement, a process crash leaves the old auth file. After replacement it leaves a complete target auth file. Startup recovery completes a valid target transaction or restores the previous encrypted payload. This is process-crash recovery; full hardware power-loss durability depends on filesystem and storage guarantees. No delete-before-rename fallback is allowed.

Codex does not acquire our operation lock. Denying write sharing and comparing snapshots reduce races, but a fully independent process may replace the directory entry while we operate. No unsupported coordination guarantee is claimed. Avoid other switchers and stop ongoing Codex operations if a concurrent-write error persists.

## Refresh watcher

A background loop polls every 800 ms. Changed candidates must produce two identical valid reads 80 ms apart. It synchronizes a matching expected identity under the core lock and records its digest only after success. Invalid/locked snapshots are retried. Global and managed isolated auth paths are covered. A refresh updates metadata/UI only and never publishes `PROFILE_CHANGED`.

The first preview does not arbitrate competing refresh rotations for the same account in multiple independent homes. Different accounts in different isolated processes are the supported target scenario. Do not rely on timestamp ordering between multiple writers.

## Isolated process model

Each profile has a UUID-addressed Codex home and VS Code user-data directory. `Code.exe --new-window --user-data-dir <path>` starts an independent environment with `CODEX_HOME` set before process creation. Shell execution is not used; only validated absolute `.exe` paths are accepted. Common API-key/access-token environment overrides are removed in child processes. Inherited organization authentication policy still applies.

A new home receives the saved login and a file-store config. Existing matching homes are synchronized back to the vault before launch rather than overwritten with an older snapshot. Settings/skills/MCP are not silently copied from the global home. The companion filters notifications by normalized home-path hash, so isolated windows ignore global switching.

## Login adapter

The program invokes the installed official native Codex executable, using a fresh access-restricted home and `-c cli_auth_credentials_store="file" login`. It does not implement OAuth, exchange tokens, call refresh endpoints, or force Logout. Login output is not captured in manager logs. The staging directory is removed only after the payload is encrypted and indexed successfully. Abandoned login staging directories are retained to avoid deleting the only credential; automatic cleanup and cancellation are follow-up work.

## Compatibility evidence

Development inspected Codex CLI `0.159.2` help and the shape/presence of the installed auth/config without printing credentials or changing the host's login. Official documentation confirms shared CLI/IDE caching, `CODEX_HOME`, supported credential stores, and built-in refresh. Windows runtime behavior and actual UI account adoption still require the manual acceptance protocol.

Workbench restart/reload commands are a VS Code adapter, not an official OpenAI account-switching API. The companion records the last applied profile before refreshing, avoiding reconnect loops. Its frame decoder rejects unknown fields, malformed JSON, unsupported versions, oversized messages, and control characters in labels.

## Deliberate scope decisions

The profile index is an atomically written JSON file rather than SQLite: this small local metadata set does not need a database or schema migration framework yet. It contains no credentials. Existing legacy formats are detected but not imported until a versioned schema has been independently verified. Logs contain allowlisted operation categories and success flags; a centralized redactor exists as additional defense, not as permission to log secrets.
