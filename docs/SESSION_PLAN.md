# Session Plan (Restart-Safe)

Last updated: 2026-03-11

## Purpose

This file is the fast resume point when a chat/session is restarted.
Use it together with `docs/TODO.md`.

## Current Workstream

Active focus:
1. Validate and harden the new auto-recovery/update flows on real package installs.
2. Improve package-install verification coverage with real distro runs.
3. Keep UI visual hierarchy stable after phase 2 rollout and collect VM feedback.
4. Move plugin-style widget registry from stage-B dual-path integration toward stage-C config normalization.

## Ground Truth Commands

Run from repo root:

```bash
cd "/Users/olivilo/Documents/Coding/Codex/20260304 WallpaperComposer"
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
- Verify quote recovery and updater behavior on Fedora (`rpm`) and Ubuntu (`deb`) package installs.
- Capture any privilege/escalation edge-cases from `pkexec` update path.

Widget target:
- Keep the two independently configurable ticker instances stable in GUI + CLI + renderer path.
- Ordering now has persistent layer Z + grid snap + overlap-safe dragging.
- Per-widget caps are wired (`news/news_ticker2/cams` refresh + FPS); keep validating defaults on low-end devices.

Layout target:
- Preserve deterministic widget layer order and avoid unreadable overlaps.

Regression target:
- Keep `native_bmp_overlay_output_hash_is_stable` passing as renderer snapshot guard.

Architecture target:
- Stage-A scaffold + Stage-B dual-path CLI integration are in place; continue staged rollout without behavior drift.

## Handoff Notes Template

When ending a work session, append short notes to commit/PR description:
- What changed
- What is still open
- Which TODO item moved to done/in progress
- Any manual verification required
