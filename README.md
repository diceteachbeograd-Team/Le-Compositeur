# Le Compositeur

Dynamic desktop compositor (Rust) by diceteachbeograd-Team.

Status: active hobby project (operator-first, Fedora-first validation path).

Temporary quality note (keep until explicit user approval):
- `Weather`, `News`, and `Static URL` are actively improved and not yet considered visually final.
- Stable mode is intentionally focused on deterministic snapshot + text workflows.

## Download (Latest)
- Direct link: [github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest](https://github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest)

### Artifact mapping
- Linux: `le-compositeur-linux-x86_64.deb` or `le-compositeur-linux-x86_64.rpm`
- Linux portable bundle: `le-compositeur-linux-x86_64.tar.gz`
- Windows: `le-compositeur-windows-x86_64.zip`
- macOS ARM: `le-compositeur-macos-arm64.dmg`

## Current Product Mode
- Stable UX path on branch `codex/fedora-first` uses:
  - background images
  - quote overlays
  - weather panel (map + metrics)
  - `News` ticker
  - `Static URL` snapshots (no live browser/video embedding)
- Experimental live video/cams are de-scoped from stable operation due to Wayland/Fedora reliability constraints.

## What's New (recent branch changes)
- `News Ticker` wording was unified to `News` in GUI-facing labels/hints.
- News ticker cadence is now decoupled from background refresh cadence.
- News ticker rendering now formats multiple headlines in one strip (`source + headline blocks`) for better readability.
- Weather minimap rework:
  - stronger city zoom
  - wind indicator moved closer to map edge area
  - higher-contrast grayscale mapping
  - CARTO Light map tiles with OpenStreetMap fallback
- Ordering remains the source of truth for placement/layering with snap-grid behavior.

## Current GUI Tabs
- `LAY Ordering`: layer toggles, z-order, drag/snap placement
- `IMG Images`: background source + timing
- `QTE Quotes`: quote source and style
- `WTH Weather`: weather source/map/panel settings
- `NWS News`: ticker source, refresh, fps, placement, width
- `URL Static`: snapshot URL source list/custom URLs and placement
- `SYS System`: runtime control, update checks, startup/integration toggles

## Build locally

Fedora/RHEL:
```bash
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 2026.03.21-1
sudo dnf upgrade -y ~/rpmbuild/RPMS/x86_64/le-compositeur-2026.03.21-1*.rpm
le-compositeur
```

Ubuntu/Debian:
```bash
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.21-1
sudo apt install ./dist/le-compositeur_2026.03.21-1_amd64.deb
le-compositeur
```

macOS (source run):
```bash
cargo run -p wc-gui
```

## Operational Notes
- Keep package release suffix unique per VM validation cycle (`YYYY.MM.DD-N`) to avoid stale-installs.
- Some widgets need internet access (`Weather`, `News`, remote image/quote/static URL sources).
- `Static URL` is snapshot-oriented by design; it is not a live browser renderer.
- Self-update flow in GUI is package-based and relies on distro package tools + auth dialog behavior.
- Default local quotes seed is packaged and auto-recovered if missing.

## Docs
- [docs/README_FULL.md](docs/README_FULL.md)
- [docs/TODO.md](docs/TODO.md)
- [docs/SESSION_PLAN.md](docs/SESSION_PLAN.md)
- [docs/RELEASE.md](docs/RELEASE.md)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/PLUGIN_REGISTRY_DRAFT.md](docs/PLUGIN_REGISTRY_DRAFT.md)
- [docs/PACKAGING.md](docs/PACKAGING.md)
- [docs/USER_CONTENT_FORMAT.md](docs/USER_CONTENT_FORMAT.md)

## Security
- [SECURITY.md](SECURITY.md)

## Support
- Litecoin: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`
