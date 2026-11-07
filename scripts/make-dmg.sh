#!/bin/sh
# Rebuilds the app and packs it as a DMG for Apple-silicon Macs: build/Snapious-<version>-arm64.dmg
# Run from the project folder:  scripts/make-dmg.sh
set -eu
cd "$(dirname "$0")/.."

scripts/bundle-app.sh

VERSION=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" build/Snapious.app/Contents/Info.plist)
OUT="build/Snapious-$VERSION-arm64.dmg"
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

cp -R build/Snapious.app "$STAGE/"
ln -s /Applications "$STAGE/Applications"
cat > "$STAGE/First-launch.txt" <<'TXT'
Snapious is not notarized by Apple. On first launch:
right-click Snapious -> Open -> Open,
or in Terminal:  xattr -cr /Applications/Snapious.app
TXT

rm -f "$OUT"
hdiutil create -volname "Snapious $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$OUT" >/dev/null
echo "Done: $OUT ($(du -h "$OUT" | cut -f1))"
