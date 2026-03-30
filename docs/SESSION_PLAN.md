# Session Plan (Restart-Safe)

Last updated: 2026-03-30

## Purpose

This file is the fast resume point when a chat/session is restarted.
Use it together with `docs/TODO.md`.

## Current Workstream

Active focus:
1. Keep cross-platform packaged builds working (Linux RPM/DEB, Windows zip, macOS ARM dmg).
2. Continue Lite profile stabilization and UI simplification for low-memory VM targets.
3. Keep wallpaper apply behavior deterministic across `Run Once`, loop start, and loop stop.
4. Ensure Fedora/Windows/macOS install tests are reproducible from GitHub release assets.
5. Keep docs/release runbook synchronized with real workflow.

Current branch state:
- Active branch is `main`.
- Cross-platform wallpaper backend matrix is active: `auto`, `noop`, `macos`, `windows`, `gnome`, `sway`, `feh`.
- GUI loop controls now include wallpaper-state snapshot lifecycle:
  - `Start Loop` / `Run Detached`: save wallpaper state
  - `Stop Loop`: restore wallpaper state
- Repeated RPM builds with same release suffix still cause stale installs; release suffix must increment per VM test build.

## Ground Truth Commands

Run from repo root:

```bash
cd "/Users/olivilo/Documents/Coding/Codex/Le-Compositeur-node24"
git status --short --branch
cargo test --all
```

If GUI flow needs manual verification:

```bash
cargo run -p wc-gui
```

If CLI flow needs manual verification:

```bash
cargo run -p wc-cli -- validate
cargo run -p wc-cli -- run --once
```

## Resume Procedure

1. Confirm branch and dirty state with `git status`.
2. Open `docs/TODO.md` and continue first unchecked item in `Now (Active Sprint)`; if all done, continue with first unchecked item in `Later`.
3. Implement one focused change.
4. Run `cargo test --all`.
5. Update `docs/TODO.md` status lines.
6. Update this file only if priorities or commands changed.

## Expected Behavior Targets (current phase)

Validation target:
- Verify Fedora VM package includes `NewsTicker` + `Static URL` tabs and no workspace `News/Cams` tabs.
- Verify ticker moves independent of background refresh cycle.
- Verify no live video windows are required in stable mode.
- Verify package version/install hash checks match latest built binaries.

Widget target:
- Keep ticker sources configurable (`news_ticker2_source`, custom URL, script ticker).
- Keep static URL workflow functional for background + text URL sources.
- Maintain stable weather/clock/quote rendering with no regressions.

Layout target:
- Preserve deterministic widget layer order and avoid unreadable overlaps.

Regression target:
- Keep `native_bmp_overlay_output_hash_is_stable` passing as renderer snapshot guard.

Architecture target:
- Stage-A scaffold + Stage-B dual-path CLI integration are in place; keep Stage-B gated via env opt-in while package/runtime fixes stabilize.

## Handoff Notes Template

When ending a work session, append short notes to commit/PR description:
- What changed
- What is still open
- Which TODO item moved to done/in progress
- Any manual verification required
