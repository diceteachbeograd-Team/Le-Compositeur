# Packaging Notes

This document tracks packaging status and next steps for distribution.

## Current status
- CI workflow exists for `fmt`, `clippy`, `test`.
- RPM skeleton exists: `packaging/rpm/wallpaper-composer.spec`.
- DEB skeleton exists: `packaging/deb/control.template`.

## What is still missing
1. Replace placeholder metadata:
- maintainer email
- GitHub URL
- release changelog entries

2. Add real install layout:
- binary path (`/usr/bin/wc-cli` currently)
- docs/license placement verified per distro policies

3. Build pipeline integration:
- GitHub workflow for RPM/DEB artifact builds
- optional signing and release upload

4. Runtime dependency verification:
- desktop utilities (`gsettings`, `swaymsg`, `feh`) are runtime-optional
- document optional dependencies in package descriptions

## Suggested order
1. Finalize project identity (org/repo/email).
2. Complete RPM path first (Fedora target).
3. Port equivalent install rules to DEB.
4. Add package smoke tests in VM matrix.

