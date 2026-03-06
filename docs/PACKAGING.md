# Packaging Notes

This document tracks packaging status and next steps for distribution.

## Current status
- CI workflow exists for `fmt`, `clippy`, `test`.
- RPM alpha spec includes CLI+GUI install metadata: `packaging/rpm/wallpaper-composer.spec`.
- DEB template includes alpha package metadata: `packaging/deb/control.template`.
- Linux desktop packaging assets are present:
  - `packaging/linux/wallpaper-composer.desktop`
  - `packaging/linux/wallpaper-composer.metainfo.xml`
  - `assets/icons/wallpaper-composer.svg`
- Alpha build scripts:
  - `scripts/build-alpha-rpm.sh`
  - `scripts/build-alpha-deb.sh`
- Cross-platform alpha artifact workflow:
  - `.github/workflows/release-alpha.yml`
  - artifacts: Linux x86_64, Windows x86_64, macOS x86_64, macOS arm64

## What is still missing
1. Final maintainer identity:
- production maintainer email
- release signing setup (optional for alpha, recommended for beta)

2. Native distro build jobs:
- Fedora COPR-style or containerized RPM build/publish
- Ubuntu/Debian native DEB build/publish

3. Runtime dependency verification:
- desktop utilities (`gsettings`, `swaymsg`, `feh`) are runtime-optional
- document optional dependencies in package descriptions

## Suggested order
1. Run alpha artifacts via `release-alpha.yml` and test on all target VMs/devices.
2. Build native RPM/DEB using scripts and validate install/remove flows.
3. Add VM smoke tests for package install + GUI launch.
4. Add signed release process for beta/stable.
