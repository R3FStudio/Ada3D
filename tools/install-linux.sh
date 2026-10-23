#!/bin/sh
# Install a release build and its launcher for the current user.
set -eu
cd "$(dirname "$0")/.."
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
install -Dm755 build/release/Ada3D "$HOME/.local/bin/Ada3D"
install -Dm644 assets/linux/Ada3D.desktop "$data_home/applications/Ada3D.desktop"
install -Dm644 assets/brand/mark.svg "$data_home/icons/hicolor/scalable/apps/Ada3D.svg"
install -Dm644 assets/brand/app-icon.png "$data_home/icons/hicolor/256x256/apps/Ada3D.png"
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$data_home/icons/hicolor" || true
fi
printf '%s\n' 'Installed Ada3D. Ensure ~/.local/bin is in your PATH.'
