# Wallpaper Composer

Dynamic wallpaper composer (Rust) by diceteachbeograd-Team.

Status: active hobby project (use at your own risk).

## Install / Run

### Option A: GitHub Release artifacts
Download from:
- Releases -> latest tag -> Assets

Current release artifacts focus on GUI app `Le Compositeur`:
- Linux: `le-compositeur-linux-x86_64.tar.gz`
- Windows: `le-compositeur-windows-x86_64.zip` (+ `LeCompositeur-windows-x86_64.exe`)
- macOS ARM: `le-compositeur-macos-arm64.dmg`

### Option B: Build locally

Fedora/RHEL:
```bash
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 1.20260307.2
sudo rpm -Uvh --replacepkgs ~/rpmbuild/RPMS/x86_64/wallpaper-composer-1.20260307.2-1*.rpm
wc-gui
```

Ubuntu/Debian:
```bash
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 1.20260307.2
sudo apt install ./dist/wallpaper-composer_1.20260307.2_amd64.deb
wc-gui
```

macOS (source run):
```bash
cargo run -p wc-gui
```

## GUI structure (current)
- `Images`: image sources + wallpaper backend apply
- `Quotes`: quote sources/order only
- `Elements`: 16:9 layout editor with draggable `Quote` and `Clock` boxes, per-element settings, and layer toggles (`Background`, `Quote`, `Clock`)
- `System`: runtime + autostart + login/boot integration toggles

## Documentation
Detailed documentation moved to:
- `docs/README_FULL.md`
- `docs/RELEASE.md`
- `docs/ARCHITECTURE.md`
- `docs/PACKAGING.md`
- `docs/USER_CONTENT_FORMAT.md`
