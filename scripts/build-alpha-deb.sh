#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-2026.03.08-2}"
PKG_DIR="${ROOT_DIR}/dist/deb-root"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    return 1
  fi
}

hint_missing_deb_deps() {
  cat >&2 <<'EOF'
hint: install Debian/Ubuntu build dependencies:
  sudo apt update
  sudo apt install -y rustc cargo dpkg-dev
EOF
}

need_file() {
  if [[ ! -f "$1" ]]; then
    echo "error: required file missing: $1" >&2
    return 1
  fi
}

cd "$ROOT_DIR"

need_cmd cargo || { hint_missing_deb_deps; exit 1; }
need_cmd dpkg-deb || { hint_missing_deb_deps; exit 1; }
need_file "assets/quotes/local/local-quotes.md" || exit 1
need_file "assets/icons/wallpaper-composer.png" || {
  cat >&2 <<'EOF'
hint: place your preferred app icon at:
  assets/icons/wallpaper-composer.png
EOF
  exit 1
}

cargo build --release -p wc-cli -p wc-gui

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN" "$PKG_DIR/usr/bin" "$PKG_DIR/usr/libexec/wallpaper-composer" "$PKG_DIR/usr/share/applications" "$PKG_DIR/usr/share/icons/hicolor/512x512/apps" "$PKG_DIR/usr/share/wallpaper-composer/quotes"

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
install -m0755 target/release/wc-gui "$PKG_DIR/usr/libexec/wallpaper-composer/wc-gui-bin"
install -m0755 packaging/linux/wc-gui-wrapper.sh "$PKG_DIR/usr/bin/wc-gui"
install -m0644 packaging/linux/wallpaper-composer.desktop "$PKG_DIR/usr/share/applications/wallpaper-composer.desktop"
install -m0644 assets/icons/wallpaper-composer.png "$PKG_DIR/usr/share/icons/hicolor/512x512/apps/wallpaper-composer.png"
install -m0644 assets/quotes/local/local-quotes.md "$PKG_DIR/usr/share/wallpaper-composer/quotes/local-quotes.md"

dpkg-deb --build "$PKG_DIR" "${ROOT_DIR}/dist/wallpaper-composer_${VERSION}_amd64.deb"

echo "DEB build complete:"
ls -lah "${ROOT_DIR}/dist/wallpaper-composer_${VERSION}_amd64.deb"
