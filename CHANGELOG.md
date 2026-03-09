# Changelog

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
