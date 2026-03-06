#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-0.1.0}"
PKG_DIR="${ROOT_DIR}/dist/deb-root"

cd "$ROOT_DIR"

cargo build --release -p wc-cli -p wc-gui

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN" "$PKG_DIR/usr/bin" "$PKG_DIR/usr/share/applications" "$PKG_DIR/usr/share/icons/hicolor/scalable/apps"

cat > "$PKG_DIR/DEBIAN/control" <<CONTROL
Package: wallpaper-composer
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Wallpaper Composer Contributors <opensource@example.com>
Description: Dynamic Linux wallpaper composer (Rust)
 Includes wc-cli and wc-gui alpha binaries.
CONTROL

install -m0755 target/release/wc-cli "$PKG_DIR/usr/bin/wc-cli"
install -m0755 target/release/wc-gui "$PKG_DIR/usr/bin/wc-gui"
install -m0644 packaging/linux/wallpaper-composer.desktop "$PKG_DIR/usr/share/applications/wallpaper-composer.desktop"
install -m0644 assets/icons/wallpaper-composer.svg "$PKG_DIR/usr/share/icons/hicolor/scalable/apps/wallpaper-composer.svg"

dpkg-deb --build "$PKG_DIR" "${ROOT_DIR}/dist/wallpaper-composer_${VERSION}_amd64.deb"

echo "DEB build complete: dist/wallpaper-composer_${VERSION}_amd64.deb"
