#!/bin/sh
# Builds the Snapious app and creates it as "build/Snapious.app".
# Requirements: full Xcode toolchain (SwiftUI macros) and accepted Xcode license:
#   sudo xcodebuild -license accept
# Run from the project folder:  scripts/bundle-app.sh
set -eu
cd "$(dirname "$0")/.."

scripts/build-ffi.sh

# Use the Xcode toolchain even if xcode-select still points to the Command Line Tools
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
      <key>CFBundleTypeName</key><string>Snapious Project</string>
      <key>CFBundleTypeExtensions</key><array><string>snapious</string></array>
      <key>CFBundleTypeRole</key><string>Editor</string>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>Sequence file</string>
      <key>CFBundleTypeExtensions</key>
      <array><string>gb</string><string>gbk</string><string>fa</string><string>fasta</string><string>fastq</string><string>fq</string></array>
      <key>CFBundleTypeRole</key><string>Viewer</string>
    </dict>
  </array>
</dict>
</plist>
PLIST

# Local ad-hoc signature so macOS launches the app without a warning (no developer certificate needed)
codesign --force --sign - "$APP" >/dev/null 2>&1 || true

echo "Done: $APP   (launch with:  open $APP)"
