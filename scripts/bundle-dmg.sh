#!/usr/bin/env bash
set -euo pipefail

# Bundle macOS .app and create styled .dmg matching drag-and-drop layout
TARGET="${1:-aarch64-apple-darwin}"
VERSION="${2:-0.1.2}"
APP_NAME="Router Switch"
OUTPUT_DIR="dist"

mkdir -p "$OUTPUT_DIR"
STAGING_DIR="$OUTPUT_DIR/staging"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"

APP_BUNDLE="$STAGING_DIR/$APP_NAME.app"
CONTENTS="$APP_BUNDLE/Contents"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

# Copy binary
if [ -f "target/$TARGET/release/router-switch" ]; then
    cp "target/$TARGET/release/router-switch" "$CONTENTS/MacOS/router-switch"
elif [ -f "target/release/router-switch" ]; then
    cp "target/release/router-switch" "$CONTENTS/MacOS/router-switch"
elif [ -f "target/$TARGET/debug/router-switch" ]; then
    cp "target/$TARGET/debug/router-switch" "$CONTENTS/MacOS/router-switch"
elif [ -f "target/debug/router-switch" ]; then
    cp "target/debug/router-switch" "$CONTENTS/MacOS/router-switch"
else
    echo "Error: router-switch binary not found!" >&2
    exit 1
fi
chmod +x "$CONTENTS/MacOS/router-switch"

# Copy Info.plist
if [ -f "assets/Info.plist" ]; then
    cp "assets/Info.plist" "$CONTENTS/Info.plist"
elif [ -f "resources/Info.plist" ]; then
    cp "resources/Info.plist" "$CONTENTS/Info.plist"
fi

# Copy icon
if [ -f "assets/icon.icns" ]; then
    cp "assets/icon.icns" "$CONTENTS/Resources/icon.icns"
elif [ -f "resources/icon.icns" ]; then
    cp "resources/icon.icns" "$CONTENTS/Resources/icon.icns"
fi

# Ad-hoc codesign the .app bundle
codesign --force --deep --sign - "$APP_BUNDLE" || true

# Package DMG
DMG_PATH="$OUTPUT_DIR/Router-Switch-$VERSION-$TARGET.dmg"
rm -f "$DMG_PATH"

if command -v create-dmg >/dev/null 2>&1; then
    echo "Creating styled DMG with create-dmg..."
    create-dmg \
        --volname "$APP_NAME" \
        --window-pos 200 120 \
        --window-size 660 400 \
        --text-size 13 \
        --icon-size 128 \
        --icon "$APP_NAME.app" 180 178 \
        --hide-extension "$APP_NAME.app" \
        --app-drop-link 480 178 \
        --no-internet-enable \
        --overwrite \
        "$DMG_PATH" \
        "$STAGING_DIR" || {
            echo "create-dmg failed, falling back to hdiutil..."
            ln -s /Applications "$STAGING_DIR/Applications"
            hdiutil create -volname "$APP_NAME" -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_PATH"
        }
else
    echo "create-dmg not found, using hdiutil..."
    ln -s /Applications "$STAGING_DIR/Applications"
    hdiutil create -volname "$APP_NAME" -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_PATH"
fi

# Also zip the .app bundle
ZIP_PATH="$OUTPUT_DIR/Router-Switch-$VERSION-macOS-$TARGET.zip"
rm -f "$ZIP_PATH"
(cd "$STAGING_DIR" && zip -r "../../$ZIP_PATH" "$APP_NAME.app")

echo "Created artifacts:"
echo "  - $DMG_PATH"
echo "  - $ZIP_PATH"
