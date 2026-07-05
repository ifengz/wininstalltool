# WinInstallTool Project

## Goal

Build a small, portable Windows installer tool for company computers. The tool uses Rust with a Slint light interface, reads local configuration, installs selected software silently, optionally installs curated local driver packs, and writes reliable install logs.

## Real Work And Waste

Company computer setup currently requires repeated manual downloads, installer clicking, driver checks, and scattered troubleshooting notes.

The smallest useful product action is: open one portable tool, choose a profile and install path, select software and driver packs, run installation, and leave a machine-readable log for success, failure, and retry.

## Users

- Installer operator: runs the tool on company computers, selects software, starts installation, retries failures, and exports logs.
- Maintainer: edits software and driver manifests, prepares offline cache, validates install commands, and reviews logs.

No account, role, or permission system is included in V1.

## V1 Scope

- Rust desktop app using Slint.
- Light theme only; no dark mode.
- Portable folder layout with external config, cache, driver packs, and logs.
- Software manifest in `config/apps.json`.
- Driver manifest in `config/drivers.json`.
- Software sources:
  - `local`: local installer file.
  - `direct_url`: fixed installer URL.
  - `github_release`: latest GitHub release asset by rule.
  - `winget`: install through Windows Package Manager.
- Silent install execution with per-app status and log capture.
- Local cache validation before install.
- Download-only mode for preparing offline cache.
- Sequential install queue with retry for failed items.
- Local install log per run.
- Optional shared log export path.
- Curated local driver-pack install through Windows built-in driver tooling.
- GUI pages for install center, driver packs, software library, logs, and settings.

## First Software List

V1 first batch candidates from the user:

| Category | Software | Initial Source Strategy | Notes |
| --- | --- | --- | --- |
| Browser | Chrome | winget or direct_url | Prefer winget first, keep local cache fallback. |
| Browser | Edge | preinstalled detection or winget | Usually preinstalled on Windows 10/11. |
| Browser | Vivaldi | winget or direct_url | Verify silent args before enabling. |
| Company/Commerce | ZiNiao Super Browser | local or direct_url | Chinese commercial installer; verify official source and silent mode. |
| Company/Commerce | Hubstudio | local or direct_url | Verify official source and silent mode. |
| Messaging | DingTalk | winget or direct_url | Verify enterprise silent install behavior. |
| Messaging | WeChat | winget or direct_url | Verify silent install behavior. |
| Messaging | Feishu | winget or direct_url | Verify silent install behavior. |
| Messaging | QQ | winget or direct_url | Verify silent install behavior. |
| Input Method | WeChat Input | local or direct_url | Verify installer type and reboot/IME activation behavior. |
| Input Method | Sogou Input | winget or direct_url | Verify installer bundle behavior. |
| Office | WPS | winget or direct_url | Verify silent args and bundle options. |
| Remote Control | Sunlogin | winget or direct_url | Needs post-install login/device binding outside V1 automation. |
| Remote Control | ToDesk | winget or direct_url | Needs post-install login/device binding outside V1 automation. |
| Remote Control | UU Remote | local or direct_url | Verify official source and silent mode. |
| Security | Huorong | local or direct_url | Security software may block automation; test carefully. |
| Cloud Drive | Quark Cloud Drive | local or direct_url | Verify silent mode. |
| Cloud Drive | Baidu Netdisk | winget or direct_url | Verify silent mode and bundle behavior. |
| Music | NetEase Cloud Music | winget or direct_url | Optional profile candidate. |
| Music | QQ Music | winget or direct_url | Optional profile candidate. |
| AI Dev Tool | Codex Desktop Client | local or direct_url | Source and silent args need confirmation. |
| AI Dev Tool | Claude Code Desktop Client | local or direct_url | Source and silent args need confirmation. |
| AI Dev Tool | Zcode Client | local or direct_url | Source and silent args need confirmation. |
| Developer Tool | Notepad++ | winget or github_release | Strong V1 candidate. |
| Image Viewer | Honeyview | winget or direct_url | Strong V1 candidate if silent args are stable. |
| Authenticator | Ente Auth | winget or github_release | Verify Windows package source. |
| Local Tool | cc switch | local or github_release | Source and packaging need confirmation. |
| Proxy Client | Clash Verge | github_release or local | Verify exact fork/distribution and config import boundary. |

V1 should treat this as a candidate list. Each item must pass source, checksum, silent install, installed-state detection, and logging verification before it becomes enabled by default.

Detailed first-pass source and installability notes live in `doc/core/software-support-matrix.md`.

## CLI Mode Decision

V1 exposes GUI only by default. The engine may internally run installer commands, winget, pnputil, and download tools, but the app must not expose an arbitrary command shell. A future CLI can be added only if it reuses the same engine and accepts explicit white-listed tool commands such as:

```text
wininstalltool.exe install --profile office --path "D:\Apps"
wininstalltool.exe download-cache --profile office
wininstalltool.exe validate-config
wininstalltool.exe export-logs --run latest
```

The future CLI would exist only for repeatable local setup, scripts, and troubleshooting. It must not become a remote-control or command-execution feature. V1 does not need this CLI unless a deployment script requires it.

## Non-Goals

- No Tauri/WebView2 UI in V1.
- No dark theme.
- No user login, role system, or permission management.
- No remote fleet management.
- No server component.
- No automatic webpage scraping for latest installers in V1.
- No automatic clicking through installer windows except a future explicit adapter.
- No generic online driver finder.
- No driver booster behavior.
- No BIOS or firmware upgrade.
- No forced replacement of working drivers.
- No software marketplace search.
- No uninstall/update management beyond what is required for install detection.

## Constraints

- Target OS: Windows 10 and Windows 11 company machines, including Pro and LTSC variants where the OS can run the required Windows tools.
- Install operations usually require administrator elevation; V1 does not implement a user permission system.
- UI must remain simple, compact, and readable on Windows display scaling.
- The tool must keep running without WebView2.
- All portable runtime data must live under the tool folder or configured company paths.
- Logs must not silently hide failures.
- Production code must not depend on mock data.
- Driver installation is limited to curated, trusted local packages.

## Proposed Folder Structure

```text
wininstalltool/
  config/
    apps.json
    drivers.json
    profiles.json
  cache/
    installers/
    downloads/
  drivers/
    packs/
  logs/
    runs/
    exports/
  src/
    app/
    engine/
    config/
    sources/
    installers/
    drivers/
    logging/
  doc/
    core/
    flow/
```

## Page Boundaries

| Route | Audience | Data | Actions | States |
| --- | --- | --- | --- | --- |
| `InstallCenter` | Installer operator | profiles, app list, cache state, install status | choose profile, choose install path, select apps, validate cache, download only, start install, retry failed | empty manifest, loading config, ready, installing, partial failure, complete |
| `DriverPacks` | Installer operator | detected machine model, driver packs, driver install status | detect model, select matching pack, install driver pack, retry failed pack | no matching pack, ready, installing, reboot required, failed |
| `SoftwareLibrary` | Maintainer | app manifest entries | add, edit, enable, disable, test command, validate manifest | valid manifest, validation errors, test passed, test failed |
| `Logs` | Installer operator, maintainer | run logs, app logs, driver logs | filter, open log folder, export log bundle | no logs, loaded, export failed |
| `Settings` | Maintainer | cache path, shared log path, network timeout, winget path | edit paths, validate paths, save settings | invalid path, saved, path unavailable |

## Data Model

### `AppManifestEntry`

- `id`: stable app identifier.
- `name`: display name.
- `enabled`: whether the app is selectable.
- `category`: UI grouping.
- `source`: package source contract.
- `install`: silent install contract.
- `detect`: installed-state detection rule.
- `cache`: optional cache policy.
- `audit`: created/updated metadata for maintainer changes.

### `PackageSource`

- `type`: `local`, `direct_url`, `github_release`, or `winget`.
- `path`: local installer path for `local`.
- `url`: fixed URL for `direct_url`.
- `repo` and `asset_pattern`: GitHub release resolver input.
- `package_id`: winget package id.
- `checksum`: optional expected hash.

### `InstallSpec`

- `installer_type`: `msi`, `inno`, `nsis`, `exe`, or `winget`.
- `silent_args`: required silent install args.
- `supports_custom_path`: boolean.
- `custom_path_arg`: optional format string for install path.
- `requires_admin`: boolean.
- `timeout_seconds`: process timeout.

### `DriverPack`

- `id`: stable driver-pack identifier.
- `name`: display name.
- `vendor`: vendor name.
- `models`: supported machine model patterns.
- `path`: local driver pack path.
- `install_mode`: `pnputil_inf_pack` in V1.
- `enabled`: whether selectable.
- `reboot_policy`: `mark_required`, `never_force`, or `unknown`.

### `RunLog`

- `run_id`: unique run identifier.
- `machine_name`: Windows machine name.
- `current_user`: current Windows user.
- `started_at`: run start time.
- `ended_at`: run end time.
- `profile_id`: selected profile.
- `items`: app and driver item results.
- `summary`: success, failure, skipped, and retry counts.

### `TaskResult`

- `item_id`: app or driver id.
- `item_type`: `app` or `driver`.
- `status`: `queued`, `running`, `success`, `failed`, `skipped`, or `reboot_required`.
- `command`: redacted command.
- `exit_code`: process exit code if available.
- `stdout_path`: captured output path.
- `stderr_path`: captured error path.
- `error_message`: user-visible failure reason.

## Interface Contracts

### `load_config(root_path)`

- Request: portable root path.
- Response: parsed app, driver, profile, and settings configuration.
- Validation failures: missing file, invalid JSON, duplicate id, unsupported source type, missing silent args.
- Side effects: none.

### `plan_install(selection, install_path)`

- Request: selected apps, selected driver packs, install path.
- Response: ordered task plan with cache requirements and admin requirements.
- Validation failures: no selected items, invalid install path, unsupported custom path.
- Side effects: none.

### `resolve_package(app_id)`

- Request: app id.
- Response: local installer path or winget package command plan.
- Validation failures: source unavailable, checksum mismatch, GitHub asset not matched, download failed.
- Side effects: may download package into cache.

### `install_app(task)`

- Request: resolved app install task.
- Response: `TaskResult`.
- Validation failures: installer missing, unsupported installer type, silent args missing.
- Side effects: starts installer process and writes output logs.

### `install_driver_pack(task)`

- Request: resolved driver-pack task.
- Response: `TaskResult`.
- Validation failures: pack path missing, no `.inf` files, model mismatch unless manually overridden.
- Side effects: invokes Windows driver tooling and writes output logs.

### `write_run_log(run)`

- Request: run state and task results.
- Response: local log paths and optional export path.
- Validation failures: log folder unavailable.
- Side effects: writes JSONL/text logs.

## Auth And Permissions

V1 has no application-level auth or user roles.

The tool should detect whether it is running with administrator rights before starting install tasks. If elevation is missing, the UI must fail loud with a clear message instead of attempting partial install.

Shared log export paths are filesystem paths only. No credentials are stored in V1.

## Async Jobs And Queues

All V1 background work is local to the running process.

- Queue: sequential local task queue.
- Lifecycle: `queued -> running -> success | failed | skipped | reboot_required`.
- Retry: user-triggered retry for failed items only.
- Timeout: per task from manifest or default settings.
- Cancellation: cancel pending tasks; running installer process should not be killed unless the operator explicitly chooses a force stop.
- Failure marking: failure records command, exit code, stderr, and visible error message.

## External Services

- `direct_url`: HTTP download from configured URL; timeout and hash validation required when checksum is configured.
- `github_release`: GitHub release metadata and asset download; unauthenticated in V1 unless later required.
- `winget`: local Windows Package Manager command execution.
- `pnputil`: local Windows driver package command execution for `.inf` driver packs.

No scraping of arbitrary download pages in V1.

## Error Handling

- Invalid manifest blocks the run before any install starts.
- Missing cache marks the item as needing download or unavailable.
- Checksum mismatch blocks that item.
- Unsupported silent install config blocks that item.
- Failed installer marks only that item failed and keeps the run log complete.
- Driver pack model mismatch blocks automatic install unless the operator explicitly overrides in a later version.
- Reboot requirement is recorded but never forces immediate reboot in V1.

## E2E Acceptance

| Flow | Seed | Steps | Expected Result | Cleanup |
| --- | --- | --- | --- | --- |
| Load valid manifest | sample `apps.json`, `drivers.json` | open app | install center shows grouped apps and no validation errors | none |
| Detect invalid manifest | duplicate app id | open app | run is blocked and validation error names duplicate id | fix sample file |
| Download-only cache | one `direct_url` app | click download-only | file appears in cache and log records checksum result | delete cached file |
| Local silent install | local test installer entry | select app and start | task reaches success or failed with exit code and captured logs | uninstall test app if installed |
| Winget plan | winget app entry | select app and start | command plan is shown and task result is logged | none |
| Driver pack match | local `.inf` pack and matching model pattern | open driver page | matching pack is selectable | none |
| Driver pack install dry run | local test pack | start driver task in dry-run/test mode | command is generated, logs are written, no hidden failure | delete test logs |
| Failed item retry | app with bad installer path | start, then retry after path fix | first run records failure, retry records success | remove test logs |
| Export logs | completed run | click export | export bundle contains JSONL summary and per-task output files | delete export bundle |

## Implementation Order

1. Create Rust workspace and project skeleton.
2. Define manifest structs and config validation.
3. Add sample `apps.json`, `drivers.json`, and `profiles.json`.
4. Implement run logging.
5. Implement package source resolution for `local`.
6. Implement silent install process runner.
7. Implement Slint light UI shell with install center and log panel.
8. Add `direct_url` downloads and checksum validation.
9. Add `winget` source support.
10. Add `github_release` source support.
11. Add driver-pack model matching.
12. Add `pnputil` driver-pack installation.
13. Add download-only mode and cache validation UI.
14. Add software library editor.
15. Package portable build and run pilot on test Windows 10/11 machines.

## Open Decisions

- First software list and required silent install parameters.
- First supported driver packs and model naming rules.
- Default cache folder policy: inside tool folder or company shared path.
- Shared log export path, if any.
- V1 should stay GUI-only unless a real deployment script requires a CLI. Any future CLI is limited to install, download-cache, validate-config, and export-logs; it must not become remote shell control.
- Whether GitHub release source needs proxy or token support in the company network.
- Confirm official download source and silent install arguments for each first-batch software item.
