# YAUZ

Yet Another Unzipper — a lightweight, cross-platform archive extraction tool built with [Tauri](https://tauri.app/) and Rust.

## Features

- **Drag & drop or double-click** to select archive files for extraction
- **Password management** — store extraction passwords (one per line), auto-deduplicated
- **Batch extraction** — process multiple archives at once with a single operation
- **Smart password retry** — automatically tries saved passwords on encrypted archives
- **Custom 7-Zip path** — point to any 7-Zip installation, supports environment variables (`%ProgramFiles%`, `$HOME`, etc.)
- **Path validation** — real-time check that the configured 7-Zip executable exists
- **Multi-language UI** — Simplified Chinese, Traditional Chinese, and British English, with system language auto-detection
- **Dark / Light mode** — follows the operating system theme automatically
- **Headless 7-Zip** — no console windows flash during extraction on Windows
- **Portable** — single executable, no installation required; config file generated alongside the exe

## Supported Archive Formats

All formats supported by 7-Zip, including: `zip`, `rar`, `7z`, `tar`, `gz`, `bz2`, `xz`, `zst`, and more.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) toolchain (1.77+)
- [7-Zip](https://www.7-zip.org/) installed somewhere on your system

### Build

```bash
cd src-tauri
cargo build --release
```

The output binary is at `src-tauri/target/release/yauz.exe` (Windows) or `src-tauri/target/release/yauz` (macOS).

### Run

Place the built binary in a folder and launch it. On first run, you will be prompted to specify the path to your 7-Zip installation.

## Project Structure

```
├── dist/                  # Frontend (HTML/CSS/JS)
│   ├── index.html         # Single-page app with i18n
│   └── fonts/             # Bundled MapleMono font
├── src-tauri/             # Rust backend (Tauri)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs        # Desktop entry point
│       └── lib.rs         # Core logic (extraction, config, commands)
├── package.json
└── README.md
```

## Configuration

On first launch, a `config.ini` file is created next to the executable:

```ini
[settings]
7zip_dir=C:\Program Files\7-Zip
language=auto

[passwords]
password1
password2
```

## License

MIT
