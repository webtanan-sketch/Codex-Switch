# Validation record

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
