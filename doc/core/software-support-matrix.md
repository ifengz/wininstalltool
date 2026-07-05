# Software Support Matrix

## Purpose

This file records the first AnySearch pass for the V1 software candidates. It is not the final `apps.json`.

An app becomes enabled by default only after these checks pass:

- official source or trusted package source is confirmed
- installer file can be downloaded or cached
- silent install works on Windows 10 and Windows 11
- installed-state detection is reliable
- install log captures exit code and output
- post-install manual steps are documented

## Source Strategy Legend

- `local`: use a curated local installer stored in the portable package/cache.
- `direct_url`: download from the official download page or fixed vendor URL.
- `github_release`: resolve the latest GitHub release asset by rule.
- `winget`: install via Windows Package Manager.
- `ms_store`: Microsoft Store package; may be less suitable for offline deployment.
- `manual_after_install`: installation can be automated, but login, binding, or configuration remains manual.

## Status Legend

- `strong_candidate`: likely suitable for early V1 verification.
- `needs_silent_test`: official source exists, but silent install must be tested.
- `manual_after_install`: installer may be automated, but useful setup needs human action.
- `high_risk`: not suitable as default V1 install until risk is resolved.
- `candidate_only`: keep in manifest later, but do not enable by default yet.

## Matrix

| App ID | Name | Category | Initial Source | Evidence | Initial Status | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `chrome` | Chrome | Browser | `direct_url` or `winget` | [Chrome Enterprise](https://chromeenterprise.google/download/) | `strong_candidate` | Prefer enterprise MSI and standard MSI silent install; keep winget fallback. |
| `edge` | Microsoft Edge | Browser | preinstalled detection, `direct_url`, or `winget` | [Edge for Business](https://www.microsoft.com/en-us/edge/business/download) | `strong_candidate` | Most Windows 10/11 machines already have Edge; first implement detection. |
| `vivaldi` | Vivaldi | Browser | `winget` or `direct_url` | [winget manifest](https://github.com/microsoft/winget-pkgs/blob/master/manifests/v/Vivaldi/Vivaldi/7.9.3970.41/Vivaldi.Vivaldi.installer.yaml) | `strong_candidate` | Winget manifest shows silent switch `--vivaldi-silent` and install location support. |
| `ziniao_superbrowser` | 紫鸟超级浏览器 | Commerce Browser | `direct_url` or `local` | [紫鸟下载页](https://www.superbrowser.com/download/) | `needs_silent_test` | Official download exists; silent install and custom path need local verification. |
| `hubstudio` | Hubstudio | Commerce Browser | `direct_url` or `local` | [Hubstudio 下载页](https://www.hubstudio.cn/download/) | `needs_silent_test` | Official download exists; support page recommends non-system disk, but silent install needs test. |
| `dingtalk` | 钉钉 | Messaging | `direct_url` or `winget` | [钉钉下载页](https://www.dingtalk.com/download) | `needs_silent_test` | Official Windows client exists; enterprise silent behavior needs verification. |
| `wechat` | 微信 | Messaging | `direct_url` or `winget` | [微信 Windows 版](https://pc.weixin.qq.com/?lang=zh-cn) | `needs_silent_test` | Official Windows download exists; silent install support is not confirmed in first pass. |
| `feishu` | 飞书 | Messaging | `direct_url` or `winget` | [飞书安装帮助](https://www.feishu.cn/hc/zh-CN/articles/360025035993-%E4%B8%8B%E8%BD%BD%E5%92%8C%E5%AE%89%E8%A3%85%E9%A3%9E%E4%B9%A6%E5%AE%A2%E6%88%B7%E7%AB%AF) | `strong_candidate` | Official help states Windows silent install uses `--command=quiet_install`. |
| `qq` | QQ | Messaging | `direct_url` or `local` | [QQ Windows](https://im.qq.com/pcqq/index.shtml), [winget issue](https://github.com/microsoft/winget-pkgs/issues/118245) | `high_risk` | Winget issue says QQ NT installer did not support silent install at that time; do not enable by default until retested. |
| `wechat_input` | 微信输入法 | Input Method | `direct_url` or `local` | [微信输入法](https://z.weixin.qq.com/) | `needs_silent_test` | Official Windows download exists; IME activation/reboot behavior needs local test. |
| `sogou_input` | 搜狗输入法 | Input Method | `direct_url` or `local` | [搜狗输入法 Windows](https://shurufa.sogou.com/windows) | `high_risk` | Input method installers commonly include extra prompts; silent and clean install must be proven before default enablement. |
| `wps` | WPS | Office | `direct_url` or `winget` | [WPS Windows](https://www.wps.com/office/windows/) | `needs_silent_test` | Official Windows download exists; bundle options and silent args need verification. |
| `sunlogin` | 向日葵远程 | Remote Control | `ms_store`, `direct_url`, or `winget` | [Microsoft Store](https://apps.microsoft.com/detail/xpddrbq2d1n7nj?hl=zh-CN) | `manual_after_install` | Install can be automated later, but account login/device binding remains manual. |
| `todesk` | ToDesk | Remote Control | `winget` or `direct_url` | [ToDesk 下载页](https://www.todesk.com/download/), [winstall](https://winstall.app/apps/Youqu.ToDesk) | `manual_after_install` | Winget package appears available; account login/device binding remains manual. |
| `uu_remote` | UU 远程 | Remote Control | `direct_url` or `ms_store` | [UU 远程官网](https://uuyc.163.com/) | `manual_after_install` | Official download exists; install may be automated, but remote-control setup remains manual. |
| `huorong` | 火绒安全 | Security | `direct_url`, `ms_store`, or `local` | [火绒个人产品](https://www.huorong.cn/person) | `needs_silent_test` | Security software can affect automation; test in isolation before default enablement. |
| `quark_cloud_drive` | 夸克网盘 | Cloud Drive | `winget` or `direct_url` | [winstall](https://winstall.app/apps/Alibaba.QuarkCloudDrive) | `needs_silent_test` | Winget package appears available; official direct source and bundle behavior still need verification. |
| `baidu_netdisk` | 百度网盘 | Cloud Drive | `winget`, `ms_store`, or `direct_url` | [winstall](https://winstall.app/apps/Baidu.BaiduNetdisk), [Microsoft Store](https://apps.microsoft.com/detail/xpdln19bmg50jx?hl=zh-CN) | `needs_silent_test` | Winget/Store paths exist; bundle behavior and silent args need verification. |
| `netease_cloud_music` | 网易云音乐 | Music | `direct_url` | [网易云音乐下载](https://music.163.com/download) | `candidate_only` | First pass found official download; winget result was a third-party client, so do not use that package by default. |
| `qq_music` | QQ 音乐 | Music | `direct_url` | [QQ 音乐下载页](https://y.qq.com/download/download.html) | `candidate_only` | Official Windows download exists; silent install needs verification. |
| `codex_desktop` | Codex Desktop Client | AI Dev Tool | `ms_store` or official download | [Microsoft Store](https://apps.microsoft.com/detail/9plm9xgg6vks?hl=en-US), [OpenAI Codex app](https://developers.openai.com/codex/app) | `candidate_only` | Official source exists; Store/MSIX deployment and size make it unsuitable as early default until install path is confirmed. |
| `claude_desktop` | Claude Desktop / Claude Code Desktop | AI Dev Tool | official download | [Claude Code desktop quickstart](https://code.claude.com/docs/en/desktop-quickstart), [Claude Desktop help](https://support.claude.com/en/articles/10065433-install-claude-desktop) | `candidate_only` | Official Windows installer exists; silent install and exact product boundary need confirmation. |
| `zcode` | Zcode Client | AI Dev Tool | official download | [ZCODE install docs](https://zcode.z.ai/en/docs/install) | `candidate_only` | Official install docs exist; silent install not confirmed. |
| `notepadpp` | Notepad++ | Developer Tool | `github_release` or `winget` | [GitHub releases](https://github.com/notepad-plus-plus/notepad-plus-plus/releases), [silent install issue](https://github.com/notepad-plus-plus/notepad-plus-plus/issues/15280) | `strong_candidate` | Releases include installers/MSI and checksums; silent install behavior is known and should be locally verified. |
| `honeyview` | Honeyview | Image Viewer | `direct_url` or `local` | [Bandisoft Honeyview](https://www.bandisoft.com/honeyview/) | `needs_silent_test` | Official page says Honeyview updates are discontinued; consider whether BandiView should replace it later. |
| `ente_auth` | Ente Auth | Authenticator | `github_release` | [Ente release](https://github.com/ente-io/ente/releases/tag/auth-v4.4.22), [Ente install help](https://ente.com/help/auth/faq/installing) | `needs_silent_test` | GitHub releases include Windows installer and checksum files; silent install needs test. |
| `cc_switch` | CC Switch | Local Tool | `github_release` | [CC Switch releases](https://github.com/farion1231/cc-switch/releases) | `candidate_only` | Official GitHub releases exist; exact Windows asset and silent install behavior need confirmation. |
| `clash_verge_rev` | Clash Verge Rev | Proxy Client | `github_release` or `winget` | [GitHub repo](https://github.com/clash-verge-rev/clash-verge-rev), [winstall](https://winstall.app/apps/ClashVergeRev.ClashVergeRev) | `needs_silent_test` | Winget package appears available; config import and proxy policy must stay out of install automation unless explicitly approved. |

## Recommended First Verification Set

Start local verification with these items because they have clearer official/package evidence and lower post-install state:

1. Chrome
2. Edge detection
3. Vivaldi
4. Feishu
5. Notepad++
6. WPS
7. DingTalk
8. WeChat
9. Huorong
10. Honeyview

Remote-control tools, AI desktop tools, proxy clients, and input methods should be tested after the install engine and logging path are stable.

## Priority App Verification Details

This section records the second AnySearch pass for the first 10 priority apps. The install commands are configuration candidates, not proof that the install has passed on a real Windows machine.

| App ID | Preferred Install Path | Candidate Silent Parameters | Detection Rule | Confidence | Evidence |
| --- | --- | --- | --- | --- | --- |
| `chrome` | Enterprise MSI or `winget` id `Google.Chrome` | MSI: `msiexec /i <installer> /qn /norestart`; winget: `winget install -e --id Google.Chrome --silent --accept-package-agreements --accept-source-agreements --disable-interactivity` | Registry uninstall display name contains `Google Chrome`; fallback path `%ProgramFiles%\Google\Chrome\Application\chrome.exe` | High | [Chrome Enterprise](https://chromeenterprise.google/download/), [winget manifest evidence](https://github.com/driftywinds/winget-pkgs/commit/dc996f8225425fe3f95c23a99d50dd5b4df618d0) |
| `edge` | Preinstalled detection first; install via Edge for Business MSI or `winget` id `Microsoft.Edge` only when missing | MSI candidate: `msiexec /i <installer> /qn /norestart`; winget candidate uses standard silent flags | Registry app path or uninstall display name contains `Microsoft Edge`; fallback path `%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe` | Medium | [Edge for Business](https://www.microsoft.com/en-us/edge/business/download), [winget install options](https://learn.microsoft.com/en-us/windows/package-manager/winget/install) |
| `vivaldi` | `winget` id `Vivaldi.Vivaldi` or direct Vivaldi installer | Direct EXE: `--vivaldi-silent --do-not-launch-chrome --system-level`; optional location: `--vivaldi-install-dir="<path>"` | Product code `Vivaldi`; fallback path `%ProgramFiles%\Vivaldi\Application\vivaldi.exe` | High | [Vivaldi winget manifest](https://github.com/microsoft/winget-pkgs/blob/master/manifests/v/Vivaldi/Vivaldi/7.9.3970.41/Vivaldi.Vivaldi.installer.yaml) |
| `feishu` | Official MSI/EXE from Feishu | Official quiet parameter: `--command=quiet_install`; MSI deployment should use standard `msiexec /i <installer> /qn /norestart` after MSI package is obtained | Registry uninstall display name contains `Feishu` or `Lark`; fallback executable path must be discovered on Windows | High for quiet arg, Medium for detection | [Feishu quiet install help](https://www.feishu.cn/hc/zh-CN/articles/360025035993-%E4%B8%8B%E8%BD%BD%E5%92%8C%E5%AE%89%E8%A3%85%E9%A3%9E%E4%B9%A6%E5%AE%A2%E6%88%B7%E7%AB%AF), [Feishu MSI deployment](https://www.feishu.cn/hc/en-US/articles/360049067543-batch-deploy-feishu-using-msi-installer) |
| `notepadpp` | GitHub release MSI preferred; winget id `Notepad++.Notepad++` acceptable | MSI: `msiexec /i <installer> /qn /norestart`; NSIS EXE fallback: `/S` | Registry uninstall display name contains `Notepad++`; fallback path `%ProgramFiles%\Notepad++\notepad++.exe` | High | [Notepad++ releases](https://github.com/notepad-plus-plus/notepad-plus-plus/releases), [silent install issue](https://github.com/notepad-plus-plus/notepad-plus-plus/issues/15280) |
| `wps` | `winget` id `Kingsoft.WPSOffice` first; direct official installer as fallback | Winget standard silent flags; direct installer switches not confirmed | Registry uninstall display name contains `WPS Office` or `WPS`; fallback executable path must be discovered on Windows | Medium | [WPS winstall](https://winstall.app/apps/Kingsoft.WPSOffice), [WPS official](https://www.wps.com/office/windows/) |
| `dingtalk` | `winget` id `Alibaba.DingTalk.Mainland` for China Mainland; official direct installer as fallback | Winget standard silent flags; direct installer switches not confirmed | Registry uninstall display name contains `DingTalk` or `钉钉` | Medium | [DingTalk Mainland winstall](https://winstall.app/apps/Alibaba.DingTalk.Mainland), [DingTalk package issue](https://github.com/microsoft/winget-pkgs/issues/362604) |
| `wechat` | `winget` id `Tencent.WeChat.Universal` or official direct installer | Winget standard silent flags; old manifest evidence shows Nullsoft/ProductCode `WeChat`; direct silent args must be tested | Product code/display name `WeChat`; fallback path `%ProgramFiles(x86)%\Tencent\WeChat\WeChat.exe` or discovered actual path | Medium | [WeChat winstall](https://winstall.app/apps/Tencent.WeChat.Universal), [WeChat winget issue](https://github.com/microsoft/winget-pkgs/issues/178063) |
| `huorong` | Official installer or local cached installer preferred | Official deployment FAQ mentions silent parameter `/S` for endpoint deployment | Registry uninstall display name contains `火绒` or `Huorong`; fallback path `%ProgramFiles(x86)%\Huorong\Sysdiag\bin\HipsMain.exe` must be verified | Medium | [Huorong install/deployment FAQ](https://www.huorong.cn/service/support/fqa/common-problem-1/installation-and-deployment/install), [Huorong official](https://www.huorong.cn/person) |
| `honeyview` | `winget` id `Bandisoft.Honeyview` or official direct installer | Third-party deployment guide and Nullsoft package evidence indicate `/S`; must be tested locally | Registry uninstall display name contains `Honeyview`; fallback path `%ProgramFiles%\Honeyview\Honeyview.exe` | Medium | [Bandisoft Honeyview](https://www.bandisoft.com/honeyview/), [Honeyview silent guide](https://www.manageengine.com/products/desktop-central/software-installation/silent_install_Honeyview-(5.50.0.0).html), [Honeyview winstall](https://winstall.app/apps/Bandisoft.Honeyview) |

## Immediate Unknowns

- Exact official direct installer URLs for several Chinese apps may be generated dynamically by their download pages.
- Silent install switches must be validated on real Windows 10/11 machines, not inferred only from web snippets.
- `winget` availability must be checked with `winget show --id <package>` on the target Windows environment.
- Store apps may be unsuitable for offline company deployment unless there is a reliable offline/package path.
- Remote-control clients require post-install account login, device binding, and security policy decisions.
- Security software and proxy clients should be tested in isolation because they can affect the installer itself.
