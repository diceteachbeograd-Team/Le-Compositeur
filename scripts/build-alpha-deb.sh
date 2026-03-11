#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-2026.03.11-5}"
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
need_file "assets/icons/le-compositeur.png" || {
  cat >&2 <<'EOF'
hint: place your preferred app icon at:
  assets/icons/le-compositeur.png
EOF
  exit 1
}

cargo build --release -p wc-cli -p wc-gui

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN" "$PKG_DIR/usr/bin" "$PKG_DIR/usr/libexec/le-compositeur" "$PKG_DIR/usr/share/applications" "$PKG_DIR/usr/share/icons/hicolor/512x512/apps" "$PKG_DIR/usr/share/le-compositeur/quotes"

cat > "$PKG_DIR/DEBIAN/control" <<CONTROL
Package: le-compositeur
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Le Compositeur Contributors <opensource@example.com>
Description: Le Compositeur dynamic desktop GUI (Rust)
 Includes wc-cli and wc-gui alpha binaries.
CONTROL

cat > "$PKG_DIR/DEBIAN/postrm" <<'POSTRM'
#!/bin/sh
set -e
if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
  for d in /home/*; do
    [ -d "$d/.config/autostart" ] || continue
    rm -f "$d/.config/autostart/le-compositeur.desktop" "$d/.config/autostart/wallpaper-composer.desktop" || true
  done
fi
exit 0
POSTRM
chmod 0755 "$PKG_DIR/DEBIAN/postrm"

install -m0755 target/release/wc-cli "$PKG_DIR/usr/bin/le-compositeur-cli"
install -m0755 target/release/wc-gui "$PKG_DIR/usr/libexec/le-compositeur/le-compositeur-bin"
install -m0755 packaging/linux/le-compositeur-wrapper.sh "$PKG_DIR/usr/bin/le-compositeur"
ln -sf le-compositeur "$PKG_DIR/usr/bin/wc-gui"
ln -sf le-compositeur-cli "$PKG_DIR/usr/bin/wc-cli"
install -m0644 packaging/linux/le-compositeur.desktop "$PKG_DIR/usr/share/applications/le-compositeur.desktop"
install -m0644 assets/icons/le-compositeur.png "$PKG_DIR/usr/share/icons/hicolor/512x512/apps/le-compositeur.png"
install -m0644 assets/quotes/local/local-quotes.md "$PKG_DIR/usr/share/le-compositeur/quotes/local-quotes.md"

dpkg-deb --build "$PKG_DIR" "${ROOT_DIR}/dist/le-compositeur_${VERSION}_amd64.deb"

echo "DEB build complete:"
ls -lah "${ROOT_DIR}/dist/le-compositeur_${VERSION}_amd64.deb"
