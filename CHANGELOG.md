# Changelog

## 0.1.2 — Local preview update

- Integrated upstream commit 61f5ee7: provision the official Codex extension in each isolated VS Code environment before opening it, with asynchronous setup and actionable installation errors.
- Preserved the local account-switch confirmation, usage-limit display, and launch environment fixes from 0.1.1.

## 0.1.1 — Local preview update

- Fixed VS Code launches inheriting Electron/Node host settings and reporting success after immediate process failure.
- Replaced the silently disabled Switch button with an explicit file-storage setup and account-switch confirmation. The existing configuration backup and recovery rules still apply.
- Added active-account quota percentages, provider-reported window lengths, reset dates/countdowns, and unavailable/stale states using the official Codex app-server read methods. No AI turn, sign-in, logout, or quota-reset redemption is requested.
- Quota reads never activate saved inactive accounts. Requests are cached for one minute and refreshed in the UI every five minutes; late responses cannot replace another account's displayed quota.

## 0.1.0 — Unreleased development preview

First implementation of the Windows account manager, opaque Codex authentication adapter, current-user DPAPI vault, recoverable atomic switching, refresh synchronization, isolated VS Code launcher, Persian Tauri desktop UI, metadata-only named-pipe companion, automated tests, and English/Persian documentation.

General availability is gated on Windows real-account acceptance, installer lifecycle checks, security review, signing, and publisher verification. No public installer release is implied by this entry.
