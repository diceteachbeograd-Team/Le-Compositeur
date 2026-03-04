# TODO and Progress

## Current Status
- MVP engine works on Fedora VM (image selection, quote selection, clock overlay, apply wallpaper).
- Rotation now supports separate timers for image and quote.
- Wallpaper fit mode support added for GNOME/Sway/feh backends.
- Text styling now configurable (quote/clock color, stroke, undercolor).

## Milestone 1: Stable CLI Core (in progress)
- [x] Base config, validate, run loop, migration.
- [x] Separate `image_refresh_seconds` and `quote_refresh_seconds`.
- [x] `wallpaper_fit_mode` support.
- [x] Text style fields (`quote_color`, `clock_color`, `text_stroke_*`, `text_undercolor`).
- [ ] Add config-level per-image include/exclude list.
- [ ] Add deterministic seed mode for reproducible rotations.

## Milestone 2: Desktop GUI Settings App
- [ ] Create `wc-gui` crate (Rust desktop app).
- [ ] Load/save config file with live validation.
- [ ] Directory/file pickers for image folder and quotes file.
- [ ] Dropdowns for source mode and presets.
- [ ] Controls for timer values, font size, positions, colors, effects.
- [ ] Preview button (`render-preview`) and apply button (`run --once`).
- [ ] Preset profiles ("minimal", "cinematic", "high-contrast").

## Milestone 3: Packaging and Install
- [ ] Fedora RPM build pipeline (`rpmbuild`/COPR-ready spec).
- [ ] Debian package baseline (`dpkg-deb`/control scripts).
- [ ] Install script for local dev (`./scripts/install-local.sh`).
- [ ] Post-install: create user config only if missing.
- [ ] Uninstall script cleanup policy.

## Milestone 4: Autostart and Multi-user
- [ ] `systemd --user` service + timer units.
- [ ] Enable/disable command in CLI.
- [ ] Multi-user system profile (`/etc/wallpaper-composer/*.toml`).
- [ ] User override precedence rules and docs.

## Milestone 5: Quality and Release
- [ ] Expand VM test matrix (Fedora GNOME/KDE, Ubuntu, Debian/openSUSE).
- [ ] Screenshot-based regression checks for overlay visibility.
- [ ] CI artifact build for Linux binaries.
- [ ] v0.2.0 alpha tag and release notes.

## Next Execution Block
1. Build `wc-gui` skeleton with settings form wired to existing schema.
2. Add folder/file pickers and save config.
3. Add quick actions: Validate, Render Preview, Run Once.
