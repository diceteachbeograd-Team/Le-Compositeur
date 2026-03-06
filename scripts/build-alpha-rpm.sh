#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-1.20260307.2}"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PKG_NAME="wallpaper-composer"
TARBALL="${PKG_NAME}-${VERSION}.tar.gz"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    return 1
  fi
}

hint_missing_fedora_deps() {
  cat >&2 <<'EOF'
hint: install Fedora build dependencies:
  sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
  rpmdev-setuptree
EOF
}

need_file() {
  if [[ ! -f "$1" ]]; then
    echo "error: required file missing: $1" >&2
    return 1
  fi
}

cd "$ROOT_DIR"

need_cmd cargo || { hint_missing_fedora_deps; exit 1; }
need_cmd rpmbuild || { hint_missing_fedora_deps; exit 1; }
need_cmd rsync || { hint_missing_fedora_deps; exit 1; }
need_file "assets/quotes/local/local-quotes.md" || exit 1
need_file "assets/icons/wallpaper-composer.png" || {
  cat >&2 <<'EOF'
hint: place your preferred app icon at:
  assets/icons/wallpaper-composer.png
EOF
  exit 1
}

cargo build --release -p wc-cli -p wc-gui

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/${PKG_NAME}-${VERSION}"
rsync -a --exclude target --exclude .git ./ "$tmpdir/${PKG_NAME}-${VERSION}/"

tar -C "$tmpdir" -czf "$tmpdir/$TARBALL" "${PKG_NAME}-${VERSION}"

mkdir -p "$HOME/rpmbuild/SOURCES"
cp "$tmpdir/$TARBALL" "$HOME/rpmbuild/SOURCES/"

rpmbuild -ba packaging/rpm/wallpaper-composer.spec \
  --define "_topdir $HOME/rpmbuild" \
  --define "version $VERSION"

echo "RPM build complete."
echo "Packages:"
find "$HOME/rpmbuild/RPMS" -name "*.rpm" -print 2>/dev/null || true
echo "Source RPMs:"
find "$HOME/rpmbuild/SRPMS" -name "*.src.rpm" -print 2>/dev/null || true
