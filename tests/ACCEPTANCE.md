# Windows acceptance protocol

Run using disposable test accounts you own, on a trusted local Windows machine. Never commit credentials or include account data in screenshots. Record OS, manager build SHA, Codex CLI version, official extension version, companion version, and each result in `docs/VALIDATION.md`. No results below are implied by the existence of this checklist.

1. **Global A → B:** save two accounts; make A active; switch B; inspect identity safely through Codex UI; confirm the companion refreshes and no browser login is needed. Verify CLI and extension agree.
2. **Refresh synchronization:** let Codex refresh B naturally; confirm last-synced metadata changes without a restart; switch A then B and confirm B still works.
3. **Separate processes:** launch A and B using Open VS Code; install both extensions in each environment; verify distinct user-data/home paths and independent account identities. Global switching must not reload either isolated window.
4. **Interrupted switch:** kill the manager at controlled pre/post-replacement test points using synthetic auth first. Verify complete old/new files and startup recovery. Use real accounts only after the synthetic crash checks pass.
5. **Logs:** enable companion debug logging; exercise login, import, refresh, switch, failure, recovery; inspect local logs offline. They must contain categories only and no tokens/auth payloads. Do not upload logs without review.
6. **Permissions:** inspect protected vault/runtime directory DACLs; verify another ordinary Windows user and a remote pipe client cannot connect/read. Test tampered DPAPI ciphertext rejection.
7. **Conflicts:** enable the known old switcher; verify explicit Disable/Continue/Cancel handling and that no extension is silently disabled.
8. **Storage/policies:** test file, keyring, auto, ephemeral, absent auth, invalid TOML, and a managed policy enforcing keyring. No unsupported backend may appear as a verified switched account.
9. **Refresh adapter:** exercise extension-host restart; then force the unsupported/failed-command branch in an extension test environment and verify window reload. Confirm profile state persists before refresh and there is no loop.
10. **Installer:** clean Windows 10/11 x64 install, WebView2 absent/present, per-user install without elevation, upgrade, uninstall, and data retention. Check tray behavior.
11. **Failure recovery:** corrupt an encrypted synthetic target, invalidate current auth, remove current auth, hold the operation lock, and simulate a competing writer. Verify errors, preservation, retry, and no partial auth file.
12. **Same-account isolation caution:** do not certify same-account concurrent refresh across multiple homes without a separate rotation investigation.

General availability requires recorded results, security review, and a signed artifact. A green CI badge covers automated checks, not this protocol.
