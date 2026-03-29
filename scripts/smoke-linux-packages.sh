#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
TAR_ASSET="${DIST_DIR}/le-compositeur-linux-x86_64.tar.gz"
DEB_ASSET="${DIST_DIR}/le-compositeur-linux-x86_64.deb"
RPM_ASSET="${DIST_DIR}/le-compositeur-linux-x86_64.rpm"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    return 1
  fi
}

need_file() {
  if [[ ! -f "$1" ]]; then
    echo "error: expected artifact missing: $1" >&2
    return 1
  fi
}

echo "==> Linux artifact smoke test: validating expected files"
need_cmd tar
need_cmd dpkg-deb
need_cmd apt-get
need_cmd docker

need_file "$TAR_ASSET"
need_file "$DEB_ASSET"
need_file "$RPM_ASSET"

echo "==> Verify tar bundle layout"
tar -tzf "$TAR_ASSET" >"${DIST_DIR}/tar-contents.txt"
grep -q "le-compositeur-linux-x86_64-bundle/bin/le-compositeur$" "${DIST_DIR}/tar-contents.txt"
grep -q "le-compositeur-linux-x86_64-bundle/bin/le-compositeur-cli$" "${DIST_DIR}/tar-contents.txt"
grep -q "le-compositeur-linux-x86_64-bundle/quotes/local-quotes.md$" "${DIST_DIR}/tar-contents.txt"

echo "==> Verify DEB metadata and install on clean Ubuntu runner"
dpkg-deb --contents "$DEB_ASSET" >/dev/null
sudo apt-get install -y "$DEB_ASSET"
wc-cli doctor >/dev/null
le-compositeur-cli doctor >/dev/null
test -f /usr/share/le-compositeur/quotes/local-quotes.md

echo "==> Verify RPM install in clean Fedora container"
docker run --rm \
  -v "${DIST_DIR}:/artifacts:ro" \
  fedora:43 \
  bash -lc '
    set -euo pipefail
    dnf install -y /artifacts/le-compositeur-linux-x86_64.rpm
    wc-cli doctor >/dev/null
    le-compositeur-cli doctor >/dev/null
    test -f /usr/share/le-compositeur/quotes/local-quotes.md
  '

echo "==> Linux artifact smoke test passed"
