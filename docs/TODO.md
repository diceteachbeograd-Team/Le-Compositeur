# TODO and Progress

Last updated: 2026-03-18

## Working Rules (must stay current)
- Update this file in every feature/fix commit.
- Keep priorities explicit: `P0` (blocking), `P1` (important), `P2` (improvement), `P3` (later).
- Every open item needs a "Done when" acceptance line.
- If session context is lost, resume from `docs/SESSION_PLAN.md`.

## Current Snapshot
- Repo branch: `codex/fedora-first`
- Strategic decision: de-scope live video/cams from core UX; focus on `NewsTicker` + `Static URL` path
- Latest published tag: `2026.03.18-2`
- Next hotfix target tag: `2026.03.18-3` (Fedora VM validation)
- Local tests: passing (`cargo test --all`)
- GUI tabs now targeted: `Ordering`, `Images`, `Quotes`, `Weather`, `NewsTicker`, `Static URL`, `System`
- Packaging artifacts implemented: Linux `rpm` + `deb`, Windows archive/installer pipeline, macOS `dmg` pipeline

## Now (Active Sprint)

- [ ] `P0` Finalize product pivot from live media to stable ticker/static URL workflow.
  Status: branch `codex/fedora-first` now removes `News`/`Cams` from workspace navigation and introduces `NewsTicker` + `Static URL` tabs; Ordering now maps to `NewsTicker` + `Static URL`; overlay runtime treats non-live static sources as snapshot path; static URL enable state is no longer force-reset each frame.
  Done when: Fedora VM package build shows no live video panes, ticker updates remain independent of BG refresh, and static URL panels are the only non-local media path.

- [ ] `P0` Harden package/release versioning so VM installs always pick up latest binaries.
  Status: repeated reuse of RPM release suffix (`2026.03.13-1`) caused false-success installs with stale binaries; hash checks proved installed files differed from latest local build.
  Done when: each VM test build uses unique package release version and post-install hash check is documented and automated.

- [ ] `P1` Build worldmonitor-style multi-source text aggregation for ticker feeds.
  Status: source catalog exists in `wc-core` but currently single-source per ticker instance dominates; new goal is feed merge/rotation rather than video windows.
  Done when: one ticker can aggregate multiple configured sources and cycle/merge lines with readable pacing.

- [ ] `P1` Document stable-mode limitations explicitly in README and release notes.
  Status: decision made to avoid live browser/video embedding in wallpaper path due to reliability risk on Wayland/Fedora.
  Done when: docs clearly state stable mode is snapshots + ticker, and "experimental live video" is opt-in only.

- [ ] `P1` Improve ticker readability and motion smoothness.
  Status: ticker now runs independently of live video, but spacing/font hierarchy still needs polish for long mixed-language headlines.
  Done when: ticker movement is visibly smoother and label/headline hierarchy remains readable at normal desktop viewing distance.

- [ ] `P1` Expand shipped source catalogs for world news and public cams.
  Status: built-in news catalog now includes world-region/country feeds and additional static/ticker presets (`Guardian World`, `Ars Technica`, `MarketWatch`, `UN News`, `ReliefWeb`) with live-vs-feed separation; CAM side still needs a larger verified public catalog beyond the current starter presets.
  Done when: operators can pick from a broad shipped world-news catalog and a broader maintained public-cam catalog without relying only on manual YouTube URL entry.

- [ ] `P0` Reproduce and fix packaged GUI freeze after `Render Preview` / action buttons on Fedora VM installs.
  Status: synchronous GUI action calls were moved to background workers with periodic repaint polling; user validation on the packaged Fedora app reports that applying settings while the loop is already running no longer crashes or deadlocks the GUI.
  Done when: packaged GUI remains clickable during and after `Validate` / `Render Preview` / `Run Once`, and no hard VM reset is required to recover.

- [ ] `P0` Rework GUI self-update flow for packaged Fedora installs.
  Status: Linux updater path now downloads the matching release asset (`.rpm`/`.deb`) from GitHub release metadata and installs that exact file via `pkexec` + package manager instead of relying on repo-visible `upgrade` commands; remaining gap is packaged Fedora VM click-path verification against a newer published tag.
  Done when: `Check Updates` + `Update Now` either complete the package upgrade end-to-end or surface a deterministic success/failure state instead of hanging after password/auth prompts.

- [ ] `P0` Disable widget runtime work when widget is disabled in `Ordering`.
  Status: `News`/`Cams` layers are now forced off in stable mode; secondary ticker was decoupled from `show_news_layer` to support no-video operation.
  Done when: disabled widgets do not fetch/render, and ticker-only mode fetches only ticker sources.

- [ ] `P1` Add packaged-install regression coverage for GUI actions and updater flow.
  Done when: there is a reproducible VM/package test path documenting `Validate`, `Render Preview`, `Run Once`, and updater behavior on Fedora package installs.

- [x] `P0` Local quote file auto-recovery in runtime (CLI + GUI).
  Done when: if `quote_source=local` and `quotes_path` is missing, app auto-creates/rebinds to valid quotes and continues without manual fix.

- [x] `P0` Keep packaged default quotes connected to user config.
  Done when: installation/init/load path guarantees a working default quotes file and config points to it (or to a recreated local copy).

- [x] `P0` Prevent GUI weather refresh freeze loop on network failure.
  Done when: failed weather refresh applies retry cooldown instead of blocking UI every frame.

- [x] `P1` Startup update check in GUI + one-click update action.
  Done when: app checks latest release on startup, shows availability in UI, and provides update button with distro-aware command fallback.

- [x] `P0` Package runtime hotfix for GUI command execution outside source tree.
  Done when: GUI uses packaged `wc-cli`/`le-compositeur-cli` without `cargo` fallback errors on VM package install.

- [x] `P0` Ensure bundled quotes are shipped in all release bundles and Linux packages.
  Done when: quote seed file is present in Linux package path and in tar/zip/dmg bundles, and missing local paths recover automatically.

- [x] `P1` Docs sync for the hotfix/release (`README`, `RELEASE`, `PACKAGING`, `CHANGELOG`, `TODO`).
  Done when: user-facing docs describe behavior, limits, and operator flow.

- [x] `P1` Multiple independent ticker instances (not only one News line).
  Done when: at least 2 separate ticker widgets can be configured and rendered independently.

- [x] `P0` Restore compact-screen reachability and runtime smoothness in tabs/widgets.
  Done when: all workspace tabs remain reachable in compact GUI mode, stream/ticker updates are not forced to 1 FPS, and image-tab BG cadence is respected again.

- [x] `P0` Harden GUI one-click self-update execution and completion reporting.
  Done when: update button launches a tracked package-manager process, reports completion/failure in GUI status, and falls back to release page with actionable reason.

- [x] `P0` Stabilize image/quote rotation cadence under high-frequency widget rendering.
  Done when: background/quote selection remains stable within a cycle even when loop tick is driven by video/ticker refresh.

- [x] `P1` Decouple ticker/video timing from user image clock settings.
  Done when: ticker shift uses auto reading-speed logic, video/camera streams enforce smooth playback floor, and loop wake interval follows `min(image_refresh_seconds, 60)` unless animation needs faster ticks.

- [ ] `P1` Remove temporary README instability warning after explicit user approval.
  Done when: user confirms `Weather`, `News`, and `Cams` are functionally acceptable and the temporary release-line warning is removed from `README.md`.

## Next (After Active Sprint)

- [x] `P1` Per-widget enable/disable + z-order + collision-safe placement rules in `Ordering`.
  Done when: users can layer widgets deterministically without accidental overlap clipping.

- [x] `P1` Performance caps per widget (`FPS`, refresh, CPU/network budget).
  Done when: each widget has configurable cap and defaults avoid overload on low-end hardware.

- [x] `P2` Expand test matrix (Fedora GNOME/KDE, Ubuntu, Debian, openSUSE).
  Done when: at least one automated run per target family is documented and reproducible.

- [x] `P2` Add screenshot-based regression checks for overlay visibility.
  Done when: render changes can be validated against known baseline snapshots.

- [x] `P1` Curated capitals cam catalog from external directories/APIs.
  Done when: ship a maintained capital-priority preset list sourced from live webcam catalogs (with health checks and fallback ranking).

## Later

- [ ] `P2` Dedicated visual system pass (layout hierarchy, spacing, typography, iconography).
  Done when: tab UX is unified and dense actions remain readable.

- [ ] `P2` Plugin-style widget registry.
  Done when: new widget types can be added without editing central render orchestration logic.

- [ ] `P3` Multi-user profile support (`/etc/...` baseline + user overrides).
  Done when: deterministic precedence is implemented and documented.

- [ ] `P3` Login/boot integration hardening per distro/display manager.
  Done when: supported matrix and rollback instructions are published.

## Done Recently

- [x] Pivoted GUI workspace from `News/Cams` to `NewsTicker/Static URL` and disabled live video/cam layers in stable mode UX path.
- [x] Decoupled `news_ticker2` enable gate from `show_news_layer` so ticker works without live video widgets.
- [x] Added overlay runtime plan support for dedicated secondary ticker process (`news_ticker2`) independent of news video window path.
- [x] Published official release `2026.03.11-8`, verified GitHub artifacts, and replaced the Fedora VM local test RPM with the official published package.
- [x] Moved GUI one-shot CLI actions (`Validate`, `Render Preview`, `Run Once`, `Migrate`, `Apply Now`) off the UI thread and added repaint polling so long renders no longer freeze the settings window.
- [x] Reworked Linux self-update logic to download the matching GitHub release package asset (`.rpm` / `.deb`) before invoking privileged local installation.
- [x] Bound secondary news ticker runtime enablement to the main `News` ordering toggle so disabled news no longer keeps ticker timing/fetch activity alive.
- [x] Validated Fedora VM source branch and locally built RPM against real user config:
  `run --once` with `apply_wallpaper=false` now completes instead of timing out; packaged CLI with all dynamic widgets enabled completed in ~13.6s, and with `Weather`/`News`/`Cams` disabled in ~3.0s.
- [x] Added single-instance run locking and `--replace-existing`.
- [x] Hardened autostart install/remove behavior and legacy cleanup.
- [x] Added weather/news rendering improvements and cams controls.
- [x] Added fixed 16:9 news size presets in GUI.
- [x] Added Linux package installation of default local quotes file.
- [x] Added runtime local quotes auto-recovery in CLI and GUI.
- [x] Added GUI weather failure cooldown to avoid repeated blocking refresh loops.
- [x] Added startup release check and one-click update action in GUI.
- [x] Added secondary independent news ticker (`show_news_ticker2`) with separate source/FPS/position/width.
- [x] Added Ordering grid snap, per-widget layer Z controls, and automatic anti-overlap drag correction.
- [x] Added per-widget performance caps (`news_refresh_seconds`, `news_ticker2_refresh_seconds`, `cams_refresh_seconds`, `cams_fps`) with runtime cache throttling.
- [x] Expanded `docs/TEST_MATRIX.md` with reproducible distro-specific automated smoke commands.
- [x] Added stable native BMP overlay snapshot/hash regression test (`native_bmp_overlay_output_hash_is_stable`).
- [x] Applied GUI visual-system pass phase 1+2 (global theme, grouped control hierarchy, tab context headers, structured status/preview panels, section-cards in Weather/News/Cams/System tabs).
- [x] Added plugin registry architecture draft (`docs/PLUGIN_REGISTRY_DRAFT.md`) with phased migration plan.
- [x] Implemented plugin registry stage-A scaffold in `wc-core/src/widget_registry.rs` (trait, registry, defaults, tests).
- [x] Implemented plugin registry stage-B dual-path integration in `wc-cli` with automatic fallback to legacy widget resolver.
- [x] Added compact-mode defaults (compact ON, preview panel OFF) to improve low-height/low-width GUI usability.
- [x] Hardened GUI CLI launcher path resolution and release-mode cargo-fallback guard.
- [x] Updated release workflow to package CLI+quotes with Linux/Windows/macOS bundles.
- [x] Restored full tab reachability via bidirectional scroll in central workspace panel.
- [x] Fixed ffmpeg frame-cache timing resolution (ms stamp) to remove accidental 1 FPS cap.
- [x] Added stream-aware background URL handling tied to `image_refresh_seconds`.
- [x] Redesigned weather minimap wind overlay (red arrow + red speed) with generated fallback map.
- [x] Removed `News` / `Cams` wallpaper mode from defaults + GUI so live media is overlay-only instead of pretending to work inside wallpaper rendering.
- [x] Reworked the weather renderer from one caption block into a map-plus-metric-tile panel with explicit location/temperature/feels/rain/wind/humidity slots.
- [x] Improved cams source fallback: YouTube thumbnail fallback without `yt-dlp` and richer source labels.
- [x] Added capital-oriented cam presets with labeled custom-entry format (`Label => URL`) for meaningful CAM ticker names.
- [x] Added async self-update process tracking in GUI (no more fire-and-forget update hang state).
- [x] Added cycle-sticky image/quote picks to keep BG/QTE stable inside one rotation window.
- [x] Switched ticker motion to reading-speed/text-length auto-shift and removed dependence on manual ticker FPS.
- [x] Enforced smooth video/camera preview floor (`>=15 FPS`) in runtime capture path.
- [x] Added host-safe overlay smoke mode via `WC_DISABLE_OVERLAY_HELPERS=1` so local tests can generate overlay state without opening visible host windows.
- [x] Added package/runtime guidance for live overlay video helpers (`mpv`, optional `yt-dlp`) and clearer CLI status when overlay video cannot start due to missing player support.
- [x] Moved built-in news source definitions into a shared catalog in `wc-core` and expanded them to shipped world-region/country sources with clear feed-vs-live-video semantics.
- [x] Added GUI news-source filtering plus overlay warning text so feed-only sources no longer masquerade as live video.
- [x] Switched the default background preset for new configs to `PlaceCats 1920x1080` and set the secondary ticker default to a world-news feed source.

## Restart Checklist (Operator)

1. Read `docs/SESSION_PLAN.md`.
2. Run `git status --short --branch`.
3. Run `cargo test --all`.
4. Execute first unchecked item from `Now (Active Sprint)`; if none remain, continue with first unchecked item from `Later`.
5. Update this file before stopping.
