# WinInstallTool

Small Windows installer tool for company computer setup.

## Current Scope

- Rust + Slint desktop app.
- Light UI only.
- GUI-only V1.
- Reads `config/apps.example.json`.
- Shows first-batch software candidates and verification state.
- Default install path can be edited or chosen with a folder picker; it only applies to software that supports custom paths.
- Does not execute real install commands yet.

## Local Development

Rust was installed locally with rustup during setup.

Run:

```powershell
cargo test
cargo run
```

## Documentation

- `doc/core/project.md`
- `doc/core/software-support-matrix.md`
