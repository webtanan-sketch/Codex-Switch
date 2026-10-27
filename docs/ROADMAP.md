# Delivery plan

## Implemented and released in v0.1.3

- Rust workspace and safe, opaque authentication adapter.
- Windows DPAPI vault and protected managed directories.
- Inter-process transaction lock, encrypted rollback, journal, atomic replacement, post-write checks.
- Stable-read polling refresh synchronization for global and isolated homes.
- Persian React/Tauri interface, system tray, setup and recovery controls.
- Native executable discovery, isolated login and independent VS Code launch.
- One-way, access-restricted Windows named-pipe broadcast server.
- VS Code companion with metadata validation, reconnect, home filtering, restart/reload fallback, conflict prompt, and restart-loop prevention.
- English/Persian documentation, automated core/protocol tests, Windows build and draft release workflows.

## Release status

Maintainer-confirmed manual acceptance scenarios are complete for the v0.1.3 release. The project remains unsigned and has not received an independent security audit; these are separate hardening tasks, not pending account-switch acceptance.
- Complete independent dependency/security review and establish code signing with a verifiable publisher identity.

## Follow-up features

- Profile rename/removal with explicit retention choices.
- Dedicated status/registration messages for isolated companion windows.
- Native filesystem events if needed; preserve stable-read and locking rules.
- Login cancellation, child lifecycle, and safe abandoned-staging cleanup.
- Validated legacy-format adapters after reviewing real schema documentation, without reading VS Code SecretStorage indirectly.
- Policy-aware diagnostics for inherited authentication overrides and managed requirements.
- Stronger same-account multi-process refresh arbitration, if supported by Codex.
- Automated VS Code Extension Development Host integration tests and accessibility review.
- Custom trusted executable picker. Persian/English desktop language selection is implemented.
