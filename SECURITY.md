# Security policy

## Supported scope

The current release is v0.1.3. The maintainer confirms that release acceptance testing is complete; the project has not received an independent security audit. Use only on a trusted Windows user account.

## Handling credentials

- Credentials never belong in source control, issues, pull requests, Actions artifacts, screenshots, or telemetry.
- Saved account payloads and one recovery snapshot are encrypted with current-user Windows DPAPI, with UI prompts disabled.
- The frontend and companion never receive token fields. Named-pipe messages contain a protocol version, profile metadata, home-path hash, and timestamp only.
- Runtime `auth.json` and temporary login homes are plaintext because the official Codex executable requires file storage. Managed directories are access restricted. Global Codex home permissions remain the user's responsibility.
- OAuth is performed by Codex. This application never manufactures, edits, or refreshes tokens itself.
- JSON/JWT identity claims are unverified local matching hints, never an authorization check.
- Non-Windows vault protection fails closed. The reversible protector in integration tests cannot be selected by the desktop application.
- Plaintext buffers use zeroization where practical; Rust JSON parsing, OS memory, swap, dumps, and Codex memory do not provide complete zeroization guarantees.
- The application does not collect telemetry or host an HTTP endpoint. Normal Codex login and usage involve OpenAI's services.

## Threat boundary

DPAPI protects saved data from offline inspection and other ordinary Windows users, subject to Windows account security. It does not protect against malicious code running as the same user, administrators, compromised dependencies, screen capture, or an unlocked session. The named-pipe DACL rejects ordinary other users and remote clients, but same-user server impersonation remains possible when the manager is absent; the protocol provides no credential access.

A process lock serializes manager mutations. Independent Codex processes do not share it. Concurrent external writers, organizational policies, and future auth formats can invalidate assumptions; the manager rejects unsupported formats and documents integration uncertainty rather than bypassing policy.

## Reporting a vulnerability

Use this repository's private GitHub vulnerability reporting if it is enabled. If it is unavailable, open a minimal issue requesting a private contact method **without vulnerability details, exploit payloads, account identifiers, tokens, auth files, or vault blobs**. Do not assume a public issue is a private channel.

Include affected manager/Codex/Windows versions, the affected component, and a sanitized description. Never send real credentials to demonstrate impact. A security report does not authorize testing against other people's accounts or machines.

## Release gates

Before general distribution: complete Windows real-account acceptance, review DPAPI/DACL/named-pipe behavior, test interruption and rollback, inspect build dependencies, verify installer upgrades/uninstalls preserve data, sign Windows artifacts, and verify publisher ownership. CI does not need or accept real credentials for tests.
