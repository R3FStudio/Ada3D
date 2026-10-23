#!/bin/sh
set -eu
cd "$(dirname "$0")"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
app_dir="$HOME/.local/opt/Ada3D"
mkdir -p "$app_dir" "$data_home/applications"
install -m755 Ada3D "$app_dir/Ada3D"
for name in LICENSE COPYRIGHT THIRD_PARTY_NOTICES.md licenses; do cp -R "$name" "$app_dir/"; done
cp -R share/icons "$data_home/"
printf '[Desktop Entry]\nType=Application\nName=Ada3D\nExec="%s/Ada3D" %%f\nIcon=Ada3D\nTerminal=false\nCategories=Graphics;2DGraphics;3DGraphics;\n' "$app_dir" > "$data_home/applications/Ada3D.desktop"
printf '%s\n' 'Ada3D installed for this user.'
