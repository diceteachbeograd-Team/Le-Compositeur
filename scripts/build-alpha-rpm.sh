#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.1.0}"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PKG_NAME="wallpaper-composer"
TARBALL="${PKG_NAME}-${VERSION}.tar.gz"

cd "$ROOT_DIR"

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

echo "RPM build complete. Check: $HOME/rpmbuild/RPMS and $HOME/rpmbuild/SRPMS"
