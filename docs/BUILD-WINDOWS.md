# Build Windows EXE locally — no GitHub Actions required

You can build from a source ZIP copied from any computer, USB drive, or local folder. No GitHub account, repository connection, git checkout, Actions runner, or GitHub token is required. Extract the entire source tree, including Cargo.lock and package-lock.json. Do not copy just the desktop folder.

## Prerequisites

Use Windows 10/11 x64 with Node.js 22+ (npm included), stable Rust with the MSVC target, Visual Studio 2022 Build Tools with Desktop development with C++ and a Windows SDK, and Microsoft Edge WebView2 Runtime. Restart PowerShell after installing tools. Check `node --version`, `npm --version`, `cargo --version`, and `rustc --version`.

## Build the application EXE

Open PowerShell in the extracted project's root directory:

```powershell
npm ci
npm run typecheck
npm test
cargo test --workspace --exclude codex-account-manager --locked
npm run desktop:build -- --no-bundle
```

The application is `target\release\codex-account-manager.exe`. Copy it to a trusted Windows x64 machine with WebView2 installed. This is the application executable, without an installer. It is unsigned unless you sign it yourself. The command does not require the Codex CLI or account credentials to compile; those are runtime requirements for signing in.

## Build an installer and VS Code companion

```powershell
npm run desktop:build
npm run extension:package
```

Outputs:

- Installer: `target\release\bundle\nsis\*-setup.exe`
- Companion: `apps\vscode-extension\codex-account-manager-0.1.0.vsix`

Install the companion through VS Code's **Extensions: Install from VSIX…** command. It is packaged locally and is not published to Marketplace.

The checked build script runs validation and creates both outputs:

```powershell
powershell -ExecutionPolicy Bypass -File .\installer\build.ps1
# Build the application EXE instead of an installer, plus the companion:
powershell -ExecutionPolicy Bypass -File .\installer\build.ps1 -ExecutableOnly
```

The execution-policy flag applies to this single PowerShell process; no machine-wide policy change is needed. Review scripts before running them.

## Internet and offline builds

“No GitHub Actions” is different from “fully offline.” First-time setup downloads npm/Cargo dependencies and Windows build tooling. Tauri's installer bundler can download NSIS/bootstrapper tools, some hosted on GitHub. If you cannot access those hosts, use `--no-bundle` to produce the application EXE, or provision the required installer tool cache separately. There is no GitHub login/API dependency in the application build itself.

For an isolated build machine, prepare identical toolchains plus npm and Cargo caches (or a vendored Cargo source tree) and Tauri bundler tools on a connected machine first. `npm ci --offline` and Cargo offline mode succeed only if all required packages already exist locally. This project does not ship a complete offline dependency/toolchain bundle.

## Troubleshooting

- `link.exe` or Windows SDK missing: install the C++ workload and SDK; reopen a developer terminal.
- `cargo`/`npm` not found: reopen PowerShell and check PATH.
- Dependency download failed: check registry/network/proxy configuration; retain the lockfiles.
- NSIS download failed: build with `--no-bundle`, or prepare the bundler cache.
- Blank desktop window: verify WebView2 is installed. Use `npm run desktop:dev` to investigate locally.
- Never add `auth.json`, vault files, or real credentials to the build folder or distributable ZIP.

## راهنمای فارسی

پوشهٔ کامل سورس را از فایل ZIP استخراج یا با فلش منتقل کنید. حساب گیت‌هاب و GitHub Actions لازم نیست. پیش‌نیازها را نصب و دستورات بالا را در ریشهٔ پروژه اجرا کنید.

برای ساخت فایل اجرایی برنامه:

```powershell
npm ci
npm run desktop:build -- --no-bundle
```

فایل خروجی: `target\release\codex-account-manager.exe`.

برای ساخت نصب‌کننده از `npm run desktop:build` استفاده کنید. نصب‌کننده در `target\release\bundle\nsis` قرار می‌گیرد. برای افزونه، دستور `npm run extension:package` را اجرا کنید. اجرای اول برای دریافت وابستگی‌ها اینترنت می‌خواهد؛ ساخت کاملاً آفلاین نیازمند آماده‌سازی قبلی کش و ابزارها است. خروجی محلی امضای دیجیتال ندارد مگر خودتان آن را امضا کنید.

زبان رابط را از فهرست **فارسی / English** در بالای برنامه انتخاب کنید؛ انتخاب ذخیره می‌شود. فارسی راست‌به‌چپ با تاریخ شمسی و انگلیسی چپ‌به‌راست با تاریخ میلادی است.

## Codex missing from an account's VS Code window

Each account now uses an explicit extension directory:
`%USERPROFILE%\.codex-hub\vscode-data\<profile-id>\extensions`.
Before opening it, the manager installs the official `openai.chatgpt` extension with the CLI belonging to the selected Code.exe and that exact user-data/extension pair. The first setup needs Marketplace access; subsequent opens reuse the installed files. Failed setup shows an error instead of silently opening an unprepared environment. Existing account credentials and editor settings are preserved.

For an offline installation, obtain the official Codex VSIX on a connected computer and run the following in PowerShell, replacing the paths/UUID with your account's values. Close that account's VS Code windows first.

```powershell
$code = 'C:\Users\YOUR_USER\AppData\Local\Programs\Microsoft VS Code\bin\code.cmd'
$data = "$env:USERPROFILE\.codex-hub\vscode-data\PROFILE_UUID"
& $code --user-data-dir $data --extensions-dir "$data\extensions" --install-extension 'D:\Downloads\codex.vsix'
```

Install the manager companion VSIX using the same flags. A companion installed only in the normal environment does not automatically exist in an isolated one.

If Extensions already shows Codex in the **same** account window, check that it is enabled for that profile. Run **View: Open View…** from the Command Palette and select Codex; a hidden Activity Bar icon is distinct from a missing installation. If activation fails, inspect **Developer: Show Running Extensions** and **Output → Log (Extension Host)** locally. Do not share credentials or unsanitized logs. Portable VS Code and custom extension policies need separate validation.

فارسی: در نسخهٔ اصلاح‌شده، افزونهٔ رسمی در مسیر همان حساب نصب و بررسی می‌شود. برای محیط‌های بدون دسترسی به Marketplace از دستور VSIX بالا استفاده کنید. اگر افزونه در همان پنجره نصب و فعال است ولی آیکن دیده نمی‌شود، از «View: Open View…» نمای Codex را باز کنید. افزونهٔ همراه را هم باید با مسیر همان حساب نصب کنید.
