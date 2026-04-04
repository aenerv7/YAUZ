---
inclusion: always
---

# YAUZ Project Context

- YAUZ stands for "Yet Another UZip".
- When referring to the full name of the project, always use "Yet Another UZip", not "Yet Another Unzipper" or any other variation.

## 7-Zip Compatibility

- YAUZ must support both standard 7-Zip and 7-Zip ZS (Zstandard edition).
- When resolving the 7-Zip executable, prefer `7z` / `7z.exe` first, then fall back to `7zz` / `7zz.exe`.
- Version string parsing must handle both formats:
  - Standard: `7-Zip 24.09 (x64)`
  - ZS: `7-Zip 26.00 ZS v1.5.7 (x64)`
- All user-facing text that mentions 7-Zip should also mention 7-Zip ZS (descriptions, error messages, setup prompts).

## Tech Stack & Environment

- Built with Rust and Tauri 2.0.
- RUSTUP_HOME: `C:\Workspace\Rust\rustup`
- CARGO_HOME: `C:\Workspace\Rust\cargo`
- When running Cargo or Rust toolchain commands, ensure these paths are respected (e.g. use `C:\Workspace\Rust\cargo\bin\cargo` if `cargo` is not on PATH).

## Release Process

- Every code change must be accompanied by a version bump before committing.
- Version must be updated in all three files: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
- Use semantic versioning (MAJOR.MINOR.PATCH).

## macOS Localization

- macOS system menus (File, Edit, Window, etc.) and WebView context menus require both `CFBundleLocalizations` in `Info.plist` AND actual `.lproj` directories with `Localizable.strings` files in `Contents/Resources/` of the `.app` bundle.
- Localization string files are stored in `src-tauri/lproj/{zh-Hans,zh-Hant,en}.lproj/Localizable.strings` and bundled automatically via `tauri.conf.json` `bundle.macOS.files`.
- `Info.plist` is configured via `src-tauri/Info.plist` and referenced in `tauri.conf.json` under `bundle.macOS.infoPlist`.

## macOS Menu Bar

- Tauri 2.0 uses the `muda` crate for menus. The default menu text is hardcoded English — macOS will NOT auto-translate it via `.lproj` files. Localization must be done in Rust code.
- The custom menu is built in `build_macos_menu()` in `lib.rs`, called via `Builder::menu()` closure. Do NOT use `setup()` hook for menus — Tauri's window creation will overwrite it with the default menu.
- The menu must replicate the full Tauri default structure (App, File, Edit, View, Window submenus) to avoid losing items. Use `WINDOW_SUBMENU_ID` for the Window submenu to preserve Tauri internal behavior.
- System language detection on macOS must use `defaults read -g AppleLanguages`, not the `LANG` environment variable. `LANG` is unreliable in GUI apps (typically `en_US.UTF-8` regardless of system language).
- When adding or modifying menu items, update all three language variants (zh-CN, zh-TW, en-GB) in the `T` struct inside `build_macos_menu()`.
- Custom menu items (like "Open File") use `MenuItem::with_id()` and emit events via `on_menu_event`. The frontend listens with `listen("menu-open-file", ...)`.
