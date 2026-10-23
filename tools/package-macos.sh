#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
bundle='dist/Ada3D.app/Contents'
mkdir -p "$bundle/MacOS" "$bundle/Resources"
cp build/release/Ada3D "$bundle/MacOS/Ada3D"
cp assets/macos/Info.plist "$bundle/Info.plist"
iconutil -c icns assets/macos/Ada3D.iconset -o "$bundle/Resources/Ada3D.icns"
for name in LICENSE COPYRIGHT THIRD_PARTY_NOTICES.md; do cp "$name" "$bundle/Resources/$name"; done
cp -R licenses "$bundle/Resources/"
plutil -lint "$bundle/Info.plist"
printf '%s\n' 'Created dist/Ada3D.app'
