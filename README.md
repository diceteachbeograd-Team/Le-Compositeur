# Le Compositeur

Dynamic desktop compositor (Rust) by diceteachbeograd-Team.

Status: active hobby project (use at your own risk).

## Download (Latest)
- Direct link: [github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest](https://github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest)

### Which file is for which OS?
- Linux: `le-compositeur-linux-x86_64.deb` or `le-compositeur-linux-x86_64.rpm`
- Windows: `LeCompositeur-windows-x86_64.exe` or `le-compositeur-windows-x86_64.zip`
- macOS ARM: `le-compositeur-macos-arm64.dmg`
- Linux portable binary: `le-compositeur-linux-x86_64`

## Build locally

Fedora/RHEL:
```bash
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 2026.03.10-4
sudo dnf install -y ~/rpmbuild/RPMS/x86_64/le-compositeur-2026.03.10-4*.rpm
le-compositeur
```

Ubuntu/Debian:
```bash
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.10-4
sudo apt install ./dist/le-compositeur_2026.03.10-4_amd64.deb
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
- `News`: news/video widget settings
- `System`: runtime, startup and integrations

## Notes
- Weather + News widgets are disabled by default after first install.
- Some widgets require internet access (`Weather`, `News`, remote image/quote sources).
- News widget size uses fixed 16:9 presets (dropdown).
- Security notes: see [SECURITY.md](SECURITY.md)

## Full docs
- [docs/README_FULL.md](docs/README_FULL.md)
- [docs/RELEASE.md](docs/RELEASE.md)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/PACKAGING.md](docs/PACKAGING.md)
- [docs/USER_CONTENT_FORMAT.md](docs/USER_CONTENT_FORMAT.md)

## Support
- XRP: `raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5`
- Litecoin: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`
