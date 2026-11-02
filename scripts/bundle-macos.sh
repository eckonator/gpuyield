#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Builds dist/GPUYield.app.
# GY_SIGN_IDENTITY / GY_SIGN_KEYCHAIN: certificate to sign with (default: ad-hoc).
# Usage: scripts/bundle-macos.sh   (GY_SKIP_BUILD=1 uses the program already in target/release)
# GY_BINARY: the program to bundle (default target/release/gpuyield); GY_ICON_TOOL: a program of
# this machine that draws the icons (default: the bundled one)
set -euo pipefail

cd "$(dirname "$0")/.."
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
APP="dist/GPUYield.app"

if [ -z "${GY_SKIP_BUILD:-}" ]; then
  cargo build --release
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "${GY_BINARY:-target/release/gpuyield}" "$APP/Contents/MacOS/GPUYield"

# The app icon is drawn by the program itself
ICONS=$(mktemp -d)
trap 'rm -rf "$ICONS"' EXIT
"${GY_ICON_TOOL:-$APP/Contents/MacOS/GPUYield}" --write-icons "$ICONS"
iconutil -c icns -o "$APP/Contents/Resources/GPUYield.icns" "$ICONS/GPUYield.iconset"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>GPUYield</string>
  <key>CFBundleDisplayName</key><string>GPUYield</string>
  <key>CFBundleIdentifier</key><string>de.eckonator.gpuyield</string>
  <key>CFBundleExecutable</key><string>GPUYield</string>
  <key>CFBundleIconFile</key><string>GPUYield</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>NSHumanReadableCopyright</key><string>© 2026 Markus Eckert · GPL-3.0-or-later</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>LSMinimumSystemVersion</key><string>10.15</string>
  <key>NSHighResolutionCapable</key><true/>
  <!-- Menu bar app: no Dock icon, not in ⌘Tab -->
  <key>LSUIElement</key><true/>
</dict>
</plist>
PLIST

# Signed with an own certificate if GY_SIGN_IDENTITY is set, otherwise ad-hoc
if [ -z "${GY_SIGN_IDENTITY:-}" ]; then
  echo "Warning: no GY_SIGN_IDENTITY, signing ad-hoc" >&2
fi
SIGN=(--force --sign "${GY_SIGN_IDENTITY:--}")
if [ -n "${GY_SIGN_KEYCHAIN:-}" ]; then
  SIGN+=(--keychain "$GY_SIGN_KEYCHAIN")
fi
codesign "${SIGN[@]}" --identifier de.eckonator.gpuyield "$APP"
codesign --verify --deep --strict "$APP"
echo "Done: $APP"
