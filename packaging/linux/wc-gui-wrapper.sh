#!/usr/bin/env bash
set -euo pipefail

GUI_BIN="/usr/libexec/wallpaper-composer/wc-gui-bin"

# In VM/remote setups, Mesa/EGL can print noisy non-fatal warnings to stderr.
# Keep startup clean by default; allow debugging by setting WC_GUI_DEBUG=1.
if [[ "${WC_GUI_DEBUG:-0}" == "1" ]]; then
  exec "${GUI_BIN}" "$@"
fi

exec "${GUI_BIN}" "$@" 2>/dev/null
