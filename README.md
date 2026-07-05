# WinInstallTool

Small Windows installer tool for company computer setup.

## Current Scope

- Rust + Slint desktop app.
- Light UI only.
- GUI-only V1.
- Reads `config/apps.example.json`.
- Shows first-batch software candidates and verification state.
- Default install path can be edited or chosen with a folder picker; it only applies to software that supports custom paths.
- Runs selected install commands on Windows through the configured `winget`, MSI, or cached EXE plan.
- Downloads cache for fixed `.exe`/`.msi` URLs and GitHub latest release assets.
- Blocks real installation on non-Windows systems.

## Local Development

Rust was installed locally with rustup during setup.

Run:

```powershell
cargo test
cargo run
```

## Windows Build On GitHub

GitHub Actions builds the Windows release package on every push to `main`.

Download it from:

```text
GitHub -> Actions -> Build Windows -> latest run -> Artifacts -> WinInstallTool-windows
```

The artifact contains:

```text
wininstalltool.exe
config/apps.example.json
cache/
logs/
```

## Documentation

- `doc/core/project.md`
- `doc/core/software-support-matrix.md`
