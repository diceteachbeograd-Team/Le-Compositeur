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
./scripts/build-alpha-rpm.sh 2026.03.08-5
sudo rpm -Uvh --replacepkgs ~/rpmbuild/RPMS/x86_64/wallpaper-composer-2026.03.08-5-1*.rpm
wc-gui
```

Ubuntu/Debian:
```bash
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.08-5
sudo apt install ./dist/wallpaper-composer_2026.03.08-5_amd64.deb
wc-gui
```

macOS (source run):
```bash
cargo run -p wc-gui
```

## GUI structure (current)
- `Ordering`: 16:9 layout frame with draggable neon boxes and layer on/off toggles (`Background`, `Quote`, `Clock`, `Weather`, `News`)
- `Images`: background source + wallpaper apply/backend
- `Quotes`: quote source/order
- `Style`: text stroke/undercolor/shadow
- `Weather`: Widget 1 settings (system location/manual location, refresh interval, current weather preview)
- `News`: Widget 2 settings (free channel presets, custom URL, FPS, audio toggle)
- `System`: runtime, startup behavior, autostart checkbox, and integration toggles

### Custom Camera Input (News Widget)
- Set `News -> Channel` to `Custom URL`.
- Paste your camera source URL (snapshot URL, MJPEG, RTSP, HLS `.m3u8`, or DASH `.mpd`).
- Camera-like sources are limited to max `1.0 FPS` (enforced), so desktop usage stays stable.
- If `ffmpeg` is available, one frame is captured from stream sources and rendered as preview image.

### Widget Size
- Weather and News widgets now have configurable `W`/`H` (width/height in pixels).
- You can change size in `Ordering` (select Weather/News), `Weather`, and `News`.

### Startup reliability
- Autostart now writes a delayed startup entry (`sleep 12`), runs one warmup cycle, then starts loop mode.
- This reduces bad wallpaper state right after login/reboot on slower desktop startup sequences.

## Documentation
Detailed documentation moved to:
- `docs/README_FULL.md`
- `docs/RELEASE.md`
- `docs/ARCHITECTURE.md`
- `docs/PACKAGING.md`
- `docs/USER_CONTENT_FORMAT.md`

## Support
If you like the project and want to support diceteachbeograd-Team:

- XRP address: `raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5`
- Litecoin address: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`

QR links:
- XRP: [open QR](https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5)
- LTC: [open QR](https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt)
