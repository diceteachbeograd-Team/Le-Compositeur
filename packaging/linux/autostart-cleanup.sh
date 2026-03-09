#!/usr/bin/env bash
set -euo pipefail

# Cleanup user-level autostart files for both current and legacy names.
for home_dir in /home/*; do
  [[ -d "${home_dir}/.config/autostart" ]] || continue
  rm -f \
    "${home_dir}/.config/autostart/le-compositeur.desktop" \
    "${home_dir}/.config/autostart/wallpaper-composer.desktop" || true
done

