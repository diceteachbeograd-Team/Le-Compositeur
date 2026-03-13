# Session Plan (Restart-Safe)

Last updated: 2026-03-13

## Purpose

This file is the fast resume point when a chat/session is restarted.
Use it together with `docs/TODO.md`.

## Current Workstream

Active focus:
1. Keep `main` stable: `News` / `Cams` disabled, weather readable, rest of app usable.
2. Continue live-media R&D only on branch `codex/live-media-rnd`.
3. Fix overlay windowing so video/ticker helpers no longer appear as normal dock/taskbar apps.
4. Restore visible multi-source CAM grid output for custom camera lists and keep per-source labels readable.
5. Rework Weather visuals only after the redesigned panel is at least as readable as the old fitted layout.
6. Keep the temporary `README.md` warning until explicit functionality approval for `Weather` / `News` / `Cams`.

Current branch state:
- `main` now force-disables `News` / `Cams` again so unfinished live-overlay work does not break normal desktop use.
- On `main`, `LAY Ordering` also keeps `News` / `Cams` unavailable: no active toggles there and no placement boxes in the position preview while the tabs remain grayed out.
- Experimental live-media work is preserved on branch `codex/live-media-rnd`.
- GUI action buttons (`Validate`, `Render Preview`, `Run Once`, `Migrate`, `Apply Now`) now run via background workers with polling/repaint instead of blocking the UI thread.
- Linux self-update now uses release assets from GitHub metadata and installs the downloaded local package file via `pkexec` + package-manager command, replacing the older repo-only upgrade attempt.
- Runtime widget gates were corrected so `News` / `Cams` are no longer forced-off by mistake, while overlay mode cleanly disables wallpaper-path rendering for those widgets.
- `System` tab now exposes a script-fed overlay ticker; runtime uses the first non-empty stdout line from the configured command.
- Schema/blueprint metadata were updated so the new overlay fields round-trip cleanly and are visible to contract consumers.
- Local smoke tests can suppress real overlay windows with `WC_DISABLE_OVERLAY_HELPERS=1`; this was added after a host-side ticker smoke opened a visible overlay outside the VM.
- Packaging/runtime notes now explicitly treat `mpv` as the live-overlay player dependency, with `yt-dlp` as a useful YouTube helper.
- Built-in news sources now live in a shared `wc-core` catalog with world-region/country coverage and GUI-side filtering.
- New default configs start with the `PlaceCats 1920x1080` image preset instead of the older random preset/local default.
- Weather on `main` has been returned to the previous fitted render path because the experimental redesign overflowed the default widget size.
- Remaining gap is to make the branch-worthy live-media path actually production-safe before any merge back from `codex/live-media-rnd`.

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
- Verify packaged GUI action buttons (`Validate`, `Render Preview`, `Run Once`, `Start Loop`) remain responsive on Fedora package installs while the loop is already active.
- Verify quote recovery and updater behavior on Fedora (`rpm`) and Ubuntu (`deb`) package installs.
- Capture any privilege/escalation edge-cases from `pkexec` / package-manager update path and ensure completion state is visible in GUI.
- Current open gap is the packaged GUI updater click-through against a newer-than-installed published release.

Widget target:
- Keep the two independently configurable ticker instances stable in GUI + CLI + renderer path.
- Ordering now has persistent layer Z + grid snap + overlap-safe dragging.
- Per-widget caps are wired (`news/news_ticker2/cams` refresh + FPS); keep validating defaults on low-end devices.
- Disabled widgets must become true runtime-off states, not just hidden overlays.
- Resolve the current mismatch between "video/camera" expectations and static-wallpaper backends by moving live media to an overlay subsystem.
- Keep the new script-fed overlay ticker usable without manual JSON edits; GUI command field is the operator surface.

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
