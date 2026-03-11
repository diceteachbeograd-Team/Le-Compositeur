# TODO and Progress

Last updated: 2026-03-11

## Working Rules (must stay current)
- Update this file in every feature/fix commit.
- Keep priorities explicit: `P0` (blocking), `P1` (important), `P2` (improvement), `P3` (later).
- Every open item needs a "Done when" acceptance line.
- If session context is lost, resume from `docs/SESSION_PLAN.md`.

## Current Snapshot
- Repo branch: `main`
- Latest tag in repo: `2026.03.11-1`
- Local tests: passing (`cargo test --all`)
- GUI tabs implemented: `Ordering`, `Images`, `Quotes`, `Weather`, `News`, `Cams`, `System`
- Packaging artifacts implemented: Linux `rpm` + `deb`, Windows archive/installer pipeline, macOS `dmg` pipeline

## Now (Active Sprint)

- [x] `P0` Local quote file auto-recovery in runtime (CLI + GUI).
  Done when: if `quote_source=local` and `quotes_path` is missing, app auto-creates/rebinds to valid quotes and continues without manual fix.

- [x] `P0` Keep packaged default quotes connected to user config.
  Done when: installation/init/load path guarantees a working default quotes file and config points to it (or to a recreated local copy).

- [x] `P0` Prevent GUI weather refresh freeze loop on network failure.
  Done when: failed weather refresh applies retry cooldown instead of blocking UI every frame.

- [x] `P1` Startup update check in GUI + one-click update action.
  Done when: app checks latest release on startup, shows availability in UI, and provides update button with distro-aware command fallback.

- [x] `P1` Docs sync for the above fixes (README, README_FULL, release notes, TODO).
  Done when: user-facing docs describe behavior, limits, and operator flow.

- [x] `P1` Multiple independent ticker instances (not only one News line).
  Done when: at least 2 separate ticker widgets can be configured and rendered independently.

## Next (After Active Sprint)

- [x] `P1` Per-widget enable/disable + z-order + collision-safe placement rules in `Ordering`.
  Done when: users can layer widgets deterministically without accidental overlap clipping.

- [x] `P1` Performance caps per widget (`FPS`, refresh, CPU/network budget).
  Done when: each widget has configurable cap and defaults avoid overload on low-end hardware.

- [x] `P2` Expand test matrix (Fedora GNOME/KDE, Ubuntu, Debian, openSUSE).
  Done when: at least one automated run per target family is documented and reproducible.

- [x] `P2` Add screenshot-based regression checks for overlay visibility.
  Done when: render changes can be validated against known baseline snapshots.

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

## Restart Checklist (Operator)

1. Read `docs/SESSION_PLAN.md`.
2. Run `git status --short --branch`.
3. Run `cargo test --all`.
4. Execute first unchecked item from `Now (Active Sprint)`; if none remain, continue with first unchecked item from `Later`.
5. Update this file before stopping.
