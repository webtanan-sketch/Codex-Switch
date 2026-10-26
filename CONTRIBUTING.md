# Contributing

Use a feature branch and a focused pull request. Discuss authentication-format changes before implementation; unknown formats must fail closed. Preserve the existing Apache-2.0 license.

## Local checks

```powershell
npm ci
npm run typecheck
npm test
npm run build
npm run check:secrets
cargo fmt --all -- --check
cargo clippy --workspace --exclude codex-account-manager --all-targets -- -D warnings
cargo test --workspace --exclude codex-account-manager
```

On Windows also build the Tauri application, package the VSIX, and perform the relevant steps in `tests/ACCEPTANCE.md`. Tests must use clearly synthetic data. Never copy an active credential into a fixture. Linux passing tests do not prove DPAPI behavior.

## Engineering rules

- UI and tray changes must use the same core switching service.
- Preserve auth payload bytes and unknown fields. Do not add token-editing or token-exchange logic.
- Keep secrets out of frontend types, command responses, Debug implementations, protocol messages, and logs.
- Use structured errors, atomic writes, restrictive permissions, and encrypted recoverability.
- Do not invoke user-controlled strings through a shell.
- Do not add telemetry, an HTTP credential endpoint, or cloud storage.
- A test-only protector must remain outside shipping application code.
- Document assumptions, validation evidence, and remaining gates. Do not label an untested Windows feature production-ready.

The intended companion publisher is `webtanan`; Marketplace ownership must be verified before publishing. Packaging a VSIX locally does not publish it.
