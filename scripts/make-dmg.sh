#!/bin/sh
# Baut die App neu und packt sie als DMG für Apple-Silicon-Macs: build/Snapious-<Version>-arm64.dmg
# Aufruf aus dem Projektordner:  scripts/make-dmg.sh
set -eu
cd "$(dirname "$0")/.."

scripts/bundle-app.sh

VERSION=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" build/Snapious.app/Contents/Info.plist)
OUT="build/Snapious-$VERSION-arm64.dmg"
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

cp -R build/Snapious.app "$STAGE/"
ln -s /Applications "$STAGE/Programme"
cat > "$STAGE/Erst-Start.txt" <<'TXT'
Snapious ist nicht von Apple notarisiert. Beim ersten Start:
Rechtsklick auf Snapious -> Öffnen -> Öffnen,
oder im Terminal:  xattr -cr /Applications/Snapious.app
TXT

rm -f "$OUT"
hdiutil create -volname "Snapious $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$OUT" >/dev/null
echo "Fertig: $OUT ($(du -h "$OUT" | cut -f1))"
