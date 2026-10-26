# Delivery plan

## Implemented in the first preview

- Rust workspace and safe, opaque authentication adapter.
- Windows DPAPI vault and protected managed directories.
- Inter-process transaction lock, encrypted rollback, journal, atomic replacement, post-write checks.
- Stable-read polling refresh synchronization for global and isolated homes.
- Persian React/Tauri interface, system tray, setup and recovery controls.
- Native executable discovery, isolated login and independent VS Code launch.
- One-way, access-restricted Windows named-pipe broadcast server.
- VS Code companion with metadata validation, reconnect, home filtering, restart/reload fallback, conflict prompt, and restart-loop prevention.
- English/Persian documentation, automated core/protocol tests, Windows build and draft release workflows.

## Required before general availability

1. Run and record all real-account scenarios in `tests/ACCEPTANCE.md` on Windows 10 and Windows 11.
2. Verify the installed official Codex extension adopts the switched account after each refresh adapter path.
3. Verify pipe DACL rejection for other users, disconnected-client cleanup, and interruption recovery on Windows.
4. Test installer WebView2 bootstrap, clean install, upgrade, uninstall, and retained user data.
5. Complete dependency/security review, signing setup, checksums, and verified publisher identity.
6. Publish a tested installer/VSIX with accurate release notes.

## Follow-up features

- Profile rename/removal with explicit retention choices.
- Dedicated status/registration messages for isolated companion windows.
- Native filesystem events if needed; preserve stable-read and locking rules.
- Login cancellation, child lifecycle, and safe abandoned-staging cleanup.
- Validated legacy-format adapters after reviewing real schema documentation, without reading VS Code SecretStorage indirectly.
- Policy-aware diagnostics for inherited authentication overrides and managed requirements.
- Stronger same-account multi-process refresh arbitration, if supported by Codex.
- Automated VS Code Extension Development Host integration tests and accessibility review.
- Optional English desktop language and custom trusted executable picker.
