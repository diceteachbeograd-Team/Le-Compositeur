# Le Compositeur

Dynamic desktop compositor (Rust) by diceteachbeograd-Team.

Status: active hobby project (use at your own risk).

Temporary note for release line `2026.03.12-3`:
- `Weather`, `News`, and `Cams` are still under active rework and are not yet considered fully reliable or visually finalized.
- This note should be removed again after explicit functionality approval.
- `main` is the stable operator line.
- On `main`, `News` and `Cams` are temporarily disabled while live-overlay development continues on branch `codex/live-media-rnd`.
- Disabled `News` / `Cams` tabs are also unavailable in `LAY Ordering`: they cannot be toggled there and no placement boxes are shown while the tabs stay grayed out.

## Download (Latest)
- Direct link: [github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest](https://github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest)

### Which file is for which OS?
- Linux: `le-compositeur-linux-x86_64.deb` or `le-compositeur-linux-x86_64.rpm`
- Linux portable bundle: `le-compositeur-linux-x86_64.tar.gz`
- Windows: `le-compositeur-windows-x86_64.zip`
- macOS ARM: `le-compositeur-macos-arm64.dmg`

## Build locally

Fedora/RHEL:
```bash
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 2026.03.12-3
sudo dnf install -y ~/rpmbuild/RPMS/x86_64/le-compositeur-2026.03.12-3*.rpm
le-compositeur
```

Ubuntu/Debian:
```bash
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.12-3
sudo apt install ./dist/le-compositeur_2026.03.12-3_amd64.deb
le-compositeur
```

macOS (source run):
```bash
cargo run -p wc-gui
```

## Current GUI tabs
- `Ordering`: layer toggles + draggable neon boxes on grayscale frame
- `Images`: background sources and wallpaper backend
- `Quotes`: quote source and quote text settings
- `Weather`: weather widget settings
- `News`: grayed out on `main` until live-overlay work is ready
- `Cams`: grayed out on `main` until live-overlay work is ready
- `System`: runtime, startup, integrations, and script-fed overlay ticker settings

## Notes
- Weather, News, and Cams widgets are disabled by default after first install.
- Default background preset for new configs is now `PlaceCats 1920x1080` (`https://placecats.com/1920/1080`).
- Some widgets require internet access (`Weather`, `News`, remote image/quote sources).
- Weather panel on `main` stays on the stable fitted layout while the more ambitious visual/icon rework continues separately.
- `News` / `Cams` are intentionally disabled on `main` until the live-overlay branch solves windowing, feed health, and visible playback correctly.
- While they are disabled, `LAY Ordering` also keeps their toggles inactive and removes their placement boxes from the position preview.
- Live-media R&D continues on branch `codex/live-media-rnd`; it is not considered production-ready for normal desktop use yet.
- System tab includes an independent overlay ticker that can be filled by any shell command; the first non-empty stdout line is rendered as the scrolling text.
- Example script ticker command: `printf 'Build %s | %s\n' "$(date +%H:%M)" "$(cat /tmp/le-compositeur-ticker.txt 2>/dev/null)"`.
- Overlay live video currently expects `mpv` on the target system; YouTube-like sources are more reliable when `yt-dlp` is installed as well.
- Ordering tab now shows a grid and snaps dragged widget positions to that grid; layer Z is configurable per widget and drag collisions auto-resolve.
- Performance caps are configurable per widget (`news_refresh_seconds`, `news_ticker2_refresh_seconds`, `cams_refresh_seconds`, `cams_fps`).
- Linux distro smoke matrix and overlay snapshot/hash regression workflow are documented in `docs/TEST_MATRIX.md`.
- Plugin-registry migration is documented in `docs/PLUGIN_REGISTRY_DRAFT.md`; stage-A scaffold is in `wc-core/src/widget_registry.rs` and stage-B dual-path is wired in `wc-cli`.
- Release bundles include both GUI + CLI binaries and packaged default quotes seed (`local-quotes.md`).
- Security notes: see [SECURITY.md](SECURITY.md)

## Full docs
- [docs/README_FULL.md](docs/README_FULL.md)
- [docs/TODO.md](docs/TODO.md)
- [docs/SESSION_PLAN.md](docs/SESSION_PLAN.md)
- [docs/RELEASE.md](docs/RELEASE.md)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/PLUGIN_REGISTRY_DRAFT.md](docs/PLUGIN_REGISTRY_DRAFT.md)
- [docs/PACKAGING.md](docs/PACKAGING.md)
- [docs/USER_CONTENT_FORMAT.md](docs/USER_CONTENT_FORMAT.md)

## Support
- Litecoin: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`
