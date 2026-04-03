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
