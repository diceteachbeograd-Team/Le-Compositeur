# Changelog

## 2026.03.11-4 - 2026-03-11
- Fixed central GUI workspace scrolling ergonomics for compact screens:
  - switched tab content viewport to two-axis scroll with disabled auto-shrink
  - restores access to long/wide settings rows in `QTE`/`WTH`/`NWS`/`CAM`/`SYS`
- Fixed frame-capture timing granularity bug causing stutter:
  - ffmpeg frame cache now uses millisecond stamps (with legacy second-stamp compatibility)
  - removes implicit 1 FPS ceiling from higher configured widget FPS values
- Improved background-source behavior:
  - `image_source=url` can now treat stream-like endpoints as independent background streams
  - refresh cadence follows `image_refresh_seconds` for background stream snapshots
  - remote image fetch uses no-cache request headers to better honor refresh cadence
- Improved weather minimap presentation and resilience:
  - minimap wind overlay now uses red direction arrow and red speed annotation
  - added generated minimap fallback when remote map fetch is unavailable
  - added minimap generation for wttr fallback payload when coordinates exist
- Improved camera source usability:
  - expanded/defaulted camera source rotation set and human-readable source labels
  - YouTube sources now have thumbnail fallback when `yt-dlp` is unavailable
  - custom camera endpoint validator now accepts `rtsp://`, `rtmp://`, `mms://`
- Increased cams FPS upper limit from `10` to `30` in core config and GUI controls.

## 2026.03.11-3 - 2026-03-11
- Fixed CI macOS release packaging regression introduced in `2026.03.11-2`:
  - moved bundled quotes seed inside app bundle resources (`Le Compositeur.app/Contents/Resources/quotes/local-quotes.md`)
  - resolves `codesign` failure (`unsealed contents present in the bundle root`)
  - restores end-to-end cross-platform release publishing for GitHub tag builds

## 2026.03.11-2 - 2026-03-11
- Fixed packaged GUI runtime command resolution:
  - GUI now searches packaged CLI binaries first (`wc-cli` / `le-compositeur-cli`, including common install paths)
  - `cargo run` fallback is disabled in release builds by default and can only be re-enabled explicitly with `WC_GUI_ALLOW_CARGO_FALLBACK=1`
  - runner exit diagnostics now capture and print CLI stderr in the GUI status panel
- Improved compact GUI defaults for smaller screens:
  - `Compact UI` remains enabled by default
  - right preview panel is now hidden by default to preserve working area
- Hardened release packaging so artifacts stay functional outside source workspaces:
  - Linux CI package/bundle now always includes both GUI + CLI binaries
  - Linux package/bundle now ships default local quotes at `/usr/share/le-compositeur/quotes/local-quotes.md` (or bundle `quotes/local-quotes.md`)
  - Windows zip now includes `LeCompositeur.exe` plus `le-compositeur-cli.exe` / `wc-cli.exe` and bundled local quotes
  - macOS app bundle now includes CLI binaries and bundled local quotes
  - removed standalone single-binary Linux/Windows release assets that lacked required CLI companion
- Stage-B plugin-registry rollout safety:
  - Stage-B path is now opt-in via `WC_WIDGET_REGISTRY_STAGE_B=1` to avoid runtime drift during package validation

## 2026.03.11-1 - 2026-03-11
- Added plugin-registry Stage-B dual-path integration in `wc-cli`:
  - new registry-backed widget resolution path using `wc-core::widget_registry`
  - built-in plugin wrappers for `quote`, `clock`, `weather`, `news`, `news_ticker2`, `cams`
  - safe automatic fallback to legacy resolver path if registry path fails
  - added tests for registry build coverage and config-to-instance mapping
- Completed GUI visual-system pass phase 2:
  - standardized section-card layout hierarchy inside `Weather`, `News`, `Cams`, `System` tabs
  - preserved phase-1 shell hierarchy (`Session`, `Actions`, `Workspace`, `Updates`)
- Consolidated version bump and release prep for tag `2026.03.11-1`:
  - updated workspace/crate versions and build script defaults
  - refreshed docs/examples to new tag/version references
  - updated metainfo release history entry
- Cumulative functional improvements shipped in this release line include:
  - local quote auto-recovery and packaged default quote binding
  - startup update check + one-click update action in GUI
  - secondary independent news ticker with separate source/FPS/position/width
  - ordering grid snap, persistent layer-z, and anti-overlap drag correction
  - per-widget performance caps and cache throttling (`news/news_ticker2/cams`)
  - renderer snapshot/hash regression guard (`native_bmp_overlay_output_hash_is_stable`)
  - expanded distro test matrix runbook (Fedora GNOME/KDE, Ubuntu, Debian, openSUSE)

## 2026.03.10-8 - 2026-03-10
- Added local quotes auto-recovery for both CLI and GUI:
  - missing local quotes paths are recreated or remapped to packaged defaults
  - recovered config is persisted automatically
- Hardened `wc-cli init` so config creation succeeds even if auto-creating quotes file fails (warning instead of hard failure).
- Added GUI startup update check against latest GitHub release and update actions:
  - `Check Updates` and `Update Now` controls
  - Linux update command attempts (`dnf`/`apt-get`/`zypper`) with release-page fallback
- Added secondary independent news ticker support (`show_news_ticker2`) with separate source/custom URL/FPS/position/width.
- Added Ordering editor improvements:
  - visible 24px grid with snap-to-grid widget dragging
  - persistent per-widget layer-z controls (`layer_z_*`) for deterministic editor stacking
  - overlap-safe drag correction to keep widgets from landing on top of each other in Ordering
- Added per-widget runtime performance caps and throttling:
  - `news_refresh_seconds` / `news_ticker2_refresh_seconds` for network refresh budget
  - `cams_refresh_seconds` + `cams_fps` for cams network/CPU decode budget
  - cache-backed widget payload reuse to avoid repeated high-frequency fetches
- Added renderer snapshot/hash regression guard test: `native_bmp_overlay_output_hash_is_stable`.
- Expanded distro verification runbook in `docs/TEST_MATRIX.md` with reproducible smoke commands for Fedora GNOME/KDE, Ubuntu, Debian, and openSUSE.
- Applied GUI visual-system pass phase 1+2:
  - unified dark-cyan theme tuning (spacing, typography, contrast)
  - grouped topbar into Session/Actions/Workspace/Updates blocks
  - added active-tab title/hint context header plus structured side/status panels
  - introduced section-card hierarchy in Weather/News/Cams/System tabs
- Added plugin registry design draft document (`docs/PLUGIN_REGISTRY_DRAFT.md`) with staged migration plan.
- Added plugin registry stage-A scaffold module in `wc-core` (`widget_registry.rs`) with registry contract tests.
- Fixed GUI weather refresh retry behavior to avoid repeated blocking refresh attempts on network failure.
- Added restart-safe planning docs and synchronized roadmap/status documentation.

## 2026.03.10-4 - 2026-03-10
- Added single-instance lock for `wc-cli run` so duplicate loop processes cannot run in parallel.
- Added `--replace-existing` runner mode and wired GUI loop start actions to replace old runners safely.
- Hardened GUI autostart install/remove logic:
  - deduplicates legacy/current autostart entries
  - autostart command replaces old loop instance before starting
- Added uninstall autostart cleanup hooks for DEB/RPM package remove.
- Updated release workflow to enforce non-draft publish mode and apply autostart cleanup hooks in Linux package artifacts.

## 2026.03.09-2 - 2026-03-09
- Added fixed 16:9 News widget size presets in GUI (dropdown, no free W/H typing).
- Enforced News widget 16:9 rendering in preview output for consistent video framing.
- Added weather geolocation cache fallback to reduce provider-rate-limit failures.
- Updated GitHub release workflow to produce cleaner OS-specific artifacts:
  - Linux: `.tar.gz`, `.deb`, `.rpm`, binary
  - Windows: `.zip`, `.exe`
  - macOS ARM: `.dmg`
- Rebranded main user-facing names to **Le Compositeur** in README, desktop entry, and metainfo.

## 2026.03.08-5 - 2026-03-08
- Added configurable Weather/News widget size controls (`W`/`H`) in `Ordering`, `Weather`, and `News` tabs.
- Weather overlay output is now compact/icon-style for cleaner on-screen display.
- News overlay text is now rendered as a single concise line.
- Added custom camera URL handling for News with enforced max `1.0 FPS` and optional `ffmpeg` frame capture.
- Updated install/build docs and release references to `2026.03.08-5`.

## 2026.03.08-3 - 2026-03-08
- Added weather fallback to `wttr.in` when IP geolocation providers are rate-limited/unavailable.
- Removed raw stream URL line from news overlay text to improve on-screen readability.
- Improved YouTube live preview candidate order using `*_live.jpg` thumbnails first.

## 2026.03.08-2 - 2026-03-08
- Added weather geolocation fallback chain (`ipapi.co` -> `ipwho.is` -> `ipinfo.io`) to avoid hard failures on provider rate limits.
- Added optional fallback from auto-location to manual geocode when `weather_location_override` is set.
- Added visual news widget preview image rendering (stream thumbnail/image when available) plus text overlay.
- Improved runtime diagnostics with `news_preview_image` path output.
- Updated version references, packaging defaults, and release docs to `2026.03.08-2`.

## 2026.03.08-1 - 2026-03-08
- Added new GUI tabs `Weather` and `News` (before `System`) with dedicated settings.
- Added weather widget configuration and live weather snapshot refresh (default 10 minutes).
- Added news widget source catalog (news/finance/tech/documentary), FPS and audio flags.
- Added explicit layer toggles for `Weather` and `News` in the `Ordering` 16:9 layout editor.
- Added Autostart checkbox workflow in `System` and hardened login startup flow (delay + warmup cycle).
- Expanded user help texts and updated onboarding documentation for easier first-time usage.

## 2026.03.07-2 - 2026-03-06
- First release-line version with scheme `YYYY.MM.DD-N`.
- Added dynamic release staging folder layout under `packaging/releases/2026.03.07-2/`.
- Unified image/quote timer handling and improved run-loop timing stability in VM usage.
- Added multi-URL support for custom image/quote endpoints (newline/`;`/`|` separators).
- Refreshed image presets and improved remote source robustness/fallback behavior.
- Added packaged local multilingual quotes defaults and expanded README formatting docs.
- Moved wallpaper settings into the Images tab and aligned Linux app icon mapping for GNOME.
- Switched Linux desktop icon packaging to explicit PNG usage to avoid stale theme overrides.

## 0.1.0 - 2026-03-04
- Initial Rust workspace with `wc-cli`, `wc-core`, `wc-render`.
- Added first CLI subcommands: `doctor`, `render-preview`.
- Added project playbook and onboarding references.
- Added `init` command to generate starter config files.
- Added config loading, image selection, and quote loading in `wc-core`.
- Connected `render-preview` to process config and produce output plus metadata sidecar.
- Added `run` command with loop mode and `--once` execution for safe testing.
- Added quote/image rotation by cycle index.
- Added optional wallpaper apply flow with backend selection (`auto`, `noop`, `gnome`, `sway`, `feh`).
- Extended config model for GUI-ready local/public source settings (presets + custom URLs).
- Added `presets` CLI command to list built-in public source options.
- Added `docs/USER_CONTENT_FORMAT.md` for user quote file formatting rules.
- Added GUI-ready text/clock layout settings in config (font sizes + positions).
- Added experimental remote source support (`image_source`/`quote_source`: `preset` or `url`) with local cache.
- Added provider-specific parsing for NASA APOD, ZenQuotes, and Quotable presets.
- Added persistent rotation state support (`rotation_use_persistent_state`, `rotation_state_file`).
- Added visible overlay rendering path via ImageMagick (`magick`/`convert`) with copy fallback.
- Added native Rust BMP overlay fallback (`native-bmp-overlay`) between ImageMagick and copy mode.
- Added `validate` command for configuration/source/backend checks.
- Added `wc-backend` crate and moved wallpaper apply logic out of `wc-cli`.
- Added explicit `preview_mode` reporting from renderer (`imagemagick-overlay` or `copy-source`).
- Added `wc-source` crate and moved remote source fetching/parsing out of `wc-cli`.
- Added `export-schema` command for GUI/tooling config schema generation.
- Added `migrate` command with automatic config backup and normalized rewrite.
- Extended schema export with GUI contract metadata (groups, labels, descriptions, enum options).
- Added conditional schema hints (`visible_when`, `enabled_when`) for GUI field logic.
- Added `ui-blueprint` command for sectioned form scaffolding and field ordering.
- Added schema `ui_widget` hints for folder/file pickers in future GUI.
- Added `preset-catalog` command and enriched preset metadata for GUI dropdowns.
- Added CI workflow (`.github/workflows/ci.yml`) for fmt/clippy/tests.
- Added packaging scaffolds for RPM and DEB plus packaging notes document.
- Added project automation helpers: `Makefile` and `justfile`.
- Added OSS repo docs: `CONTRIBUTING`, `CODE_OF_CONDUCT`, `SECURITY`, issue/PR templates.
