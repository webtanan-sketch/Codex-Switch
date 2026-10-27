# Validation record

## Local Windows update — 0.1.2

Updated on Windows x64, 2026-10-27, to upstream commit `61f5ee7420ff67178f8b35e8cc8883319525a8ec`, preserving the local 0.1.1 fixes. The build includes local changes; the upstream SHA alone does not identify its complete source.

- Rust core suite: 47 tests passed, including the three upstream isolated-extension setup tests and the existing launch-environment regressions.
- TypeScript type checking and all 7 desktop/companion tests passed.
- Rust formatting, core Clippy with warnings denied, and the source credential-pattern scan passed.
- Optimized native x64 desktop compilation and NSIS installer generation passed for version 0.1.2.
- Existing account-switch and quota UI behavior was preserved during conflict resolution. No real-account switch or installation of 0.1.2 was performed during this build update.


## Local Windows update — 0.1.1

Validated on Windows x64, 2026-10-26, with Rust 1.99.1, Node 22.20.0, and native Codex CLI 0.162.0-alpha.17.2:

- Reproduced the inherited `ELECTRON_RUN_AS_NODE` / `NODE_OPTIONS` launch failures with Windows child-process tests; both failed before the fix and passed afterward.
- Rust core suite: 44 tests passed, including actual Windows DPAPI, guarded replacement, quota parsing/framing, account association, and named-pipe tests.
- Desktop formatting helpers and companion protocol: 7 tests passed.
- Moli with `--layout`: quota rendering, setup confirmation/cancel, switching, VS Code action, unavailable quota, wrong-account quota, and late-response protection passed using synthetic metadata only.
- Moli did not apply the CDP viewport override to its real layout viewport. Edge was used only for 760px/1120px viewport and visual parity checks; both passed without horizontal overflow or JavaScript errors.
- Real `Code.exe` GUI launch passed using the core launcher, a dedicated user-data directory, and an empty test Codex home. No saved account was selected for that GUI probe.
- The optional official app-server quota read succeeded for the currently registered login. Identity and `config.toml` remained unchanged; no AI turn, login/logout, or quota-reset action was requested. No account identifiers or authentication data are recorded here.
- The locally built NSIS installer upgraded the per-user installation from 0.1.0 to 0.1.1. Both saved profiles and encrypted vault files were preserved; an encrypted-profile/settings backup was taken before replacement. The installed executable matches the source build after normalizing Tauri's three-byte NSIS bundle marker.
- The installed 0.1.1 window was inspected through accessibility: the version, active-account usage panel, weekly limit, reset countdown, and provider-omitted hourly-limit notice were present, with no quota-read error.

This does not establish completed real-account A → B acceptance, concurrent token-rotation behavior, cross-user ACL testing, code signing, or general availability. The companion remains protocol-compatible at 0.1.0.

## Development environment

- Source development: Linux container, 2026-10-26.
- Inspected installed Codex CLI: `0.159.2`; inspected login help and auth/config shape only. No host credentials were printed, copied into this repository, or modified.
- Rust toolchain: stable `1.99.1`; Node/npm managed runtime.
- No interactive Windows desktop, real-account switching, or signed installer testing was performed in this development environment.

## Automated checks

| Check | Result |
| --- | --- |
| Rust core tests on Linux | Passed; 32 tests after atomic-write, auth-mode, and active-profile regression coverage |
| Rust core Clippy with warnings denied | Passed |
| Rust formatting | Passed |
| Windows-target core and complete Tauri desktop compile checks | Passed for `x86_64-pc-windows-gnu`; compile checks only, no desktop launched |
| TypeScript strict type checking | Passed across all workspaces |
| VS Code protocol tests | Passed; 4 tests |
| Vite production frontend build | Passed |
| Companion TypeScript build and VSIX packaging | Passed locally |
| Source credential-pattern scan | Passed; heuristic scan, not a complete security audit |
| Windows core suite | Passed in [Actions run 40307226917](https://github.com/webtanan-sketch/Codex-Switch/actions/runs/40307226917) on commit `12bcc73`: 34 tests, including real DPAPI/tamper checks, guarded replacement, and duplex named-pipe reconnect |
| Cross-user DACL / remote pipe rejection | Requires manual Windows evidence |
| Tauri Windows installer build and VSIX packaging | Passed on commit `12bcc73` in [Actions run 40307226917](https://github.com/webtanan-sketch/Codex-Switch/actions/runs/40307226917); unsigned preview artifacts uploaded |
| Real Codex/VS Code account adoption | Not run; requires manual acceptance |
| Windows 10/11 installation, upgrade, uninstall | Not run |
| Code signing / Marketplace publication | Not configured / not published |

The Linux transaction suite uses an injected test-only protector; Windows uses actual DPAPI for the same scenarios. A successful Windows-target compile does not validate DPAPI runtime semantics or filesystem permissions. Injected interruption tests exercise transaction boundaries and flushed temporary writes; they do not establish physical power-loss durability.

Follow `tests/ACCEPTANCE.md` before claiming general availability. Update this record with the build SHA and test evidence after any Windows run; do not change “not run” to “passed” based only on generated source or pipeline configuration.

The Windows CI run above caught and verified a fix for guarded file replacement: an existing Windows auth destination now uses `ReplaceFileW` rather than a generic move. It also verifies read/write pipe clients can connect to the broadcast-only application protocol. The subsequent auth-mode rejection test increases the expected totals to 32 on Linux and 35 on Windows; its latest CI result must be checked separately.
