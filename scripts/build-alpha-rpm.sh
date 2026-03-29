#!/usr/bin/env bash
set -euo pipefail

RELEASE_TAG="${1:-2026.03.13-1}"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PKG_NAME="le-compositeur"
RPM_VERSION="${RELEASE_TAG%-*}"
RPM_RELEASE="${RELEASE_TAG##*-}"
TARBALL="${PKG_NAME}-${RPM_VERSION}.tar.gz"

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
need_file "assets/icons/le-compositeur.png" || {
  cat >&2 <<'EOF'
hint: place your preferred app icon at:
  assets/icons/le-compositeur.png
EOF
  exit 1
}

if [[ "${WC_SKIP_CARGO_BUILD:-0}" != "1" ]]; then
  cargo build --release -p wc-cli -p wc-gui
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/${PKG_NAME}-${RPM_VERSION}"
rsync -a --exclude target --exclude .git ./ "$tmpdir/${PKG_NAME}-${RPM_VERSION}/"

tar -C "$tmpdir" -czf "$tmpdir/$TARBALL" "${PKG_NAME}-${RPM_VERSION}"

mkdir -p "$HOME/rpmbuild/SOURCES"
cp "$tmpdir/$TARBALL" "$HOME/rpmbuild/SOURCES/"

RPMBUILD_ARGS=(
  -ba packaging/rpm/le-compositeur.spec
  --define "_topdir $HOME/rpmbuild"
  --define "version $RPM_VERSION"
  --define "release $RPM_RELEASE"
)

if [[ "${WC_RPM_NODEPS:-0}" == "1" ]]; then
  RPMBUILD_ARGS+=(--nodeps)
fi

rpmbuild "${RPMBUILD_ARGS[@]}"

echo "RPM build complete."
echo "Packages:"
find "$HOME/rpmbuild/RPMS" -name "*.rpm" -print 2>/dev/null || true
echo "Source RPMs:"
find "$HOME/rpmbuild/SRPMS" -name "*.src.rpm" -print 2>/dev/null || true
