#!/bin/sh
# Baut die Snapious-App und legt sie als "build/Snapious.app" an.
# Voraussetzung: vollständige Xcode-Toolchain (SwiftUI-Makros) und akzeptierte Xcode-Lizenz:
#   sudo xcodebuild -license accept
# Aufruf aus dem Projektordner:  scripts/bundle-app.sh
set -eu
cd "$(dirname "$0")/.."

scripts/build-ffi.sh

# Xcode-Toolchain verwenden, auch wenn xcode-select noch auf die Command Line Tools zeigt
if [ -d /Applications/Xcode.app/Contents/Developer ]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi

(cd app && swift build -c release --product Snapious)
BIN=app/.build/release/Snapious

APP=build/Snapious.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/Snapious"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Snapious</string>
  <key>CFBundleDisplayName</key><string>Snapious</string>
  <key>CFBundleIdentifier</key><string>de.snapious.app</string>
  <key>CFBundleExecutable</key><string>Snapious</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Snapious-Projekt</string>
      <key>CFBundleTypeExtensions</key><array><string>snapious</string></array>
      <key>CFBundleTypeRole</key><string>Editor</string>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>Sequenzdatei</string>
      <key>CFBundleTypeExtensions</key>
      <array><string>gb</string><string>gbk</string><string>fa</string><string>fasta</string><string>fastq</string><string>fq</string></array>
      <key>CFBundleTypeRole</key><string>Viewer</string>
    </dict>
  </array>
</dict>
</plist>
PLIST

# Lokale Ad-hoc-Signatur, damit macOS die App ohne Warnung startet (kein Entwicklerzertifikat nötig)
codesign --force --sign - "$APP" >/dev/null 2>&1 || true

echo "Fertig: $APP   (starten mit:  open $APP)"
