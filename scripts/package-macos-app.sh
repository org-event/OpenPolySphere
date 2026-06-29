#!/usr/bin/env bash
# Build OpenPolySphere.app + standard macOS .dmg (drag to Applications).
#
# Usage: ./scripts/package-macos-app.sh <version> [output-dir]
# Outputs:
#   dist/openpolysphere-<version>-macos-<arch>.dmg  (recommended)
#   dist/openpolysphere-<version>-macos-<arch>.zip  (fallback)

set -euo pipefail

VERSION="${1:?usage: package-macos-app.sh <version> [output-dir]}"
OUT_DIR="${2:-dist}"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH_LABEL="x64" ;;
  *) ARCH_LABEL="$ARCH" ;;
esac
ARTIFACT_BASE="openpolysphere-${VERSION}-macos-${ARCH_LABEL}"
WORK="$OUT_DIR/.pack-${ARTIFACT_BASE}"
APP="$WORK/OpenPolySphere.app"
DMG="$OUT_DIR/${ARTIFACT_BASE}.dmg"
ZIP="$OUT_DIR/${ARTIFACT_BASE}.zip"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD="$ROOT/target/release"
APP_BUILD="$BUILD/openpolysphere"

cd "$ROOT"

if [[ ! -x "$BUILD/translator" ]]; then
  echo "missing $BUILD/translator — run: cargo build --release -p translator" >&2
  exit 1
fi
if [[ ! -x "$APP_BUILD" ]]; then
  echo "missing $APP_BUILD — run: cargo build --release -p openpolysphere-app" >&2
  exit 1
fi

if [[ ! -d "$BUILD/PolySphereSpeech.app" ]]; then
  echo "missing $BUILD/PolySphereSpeech.app — rebuild translator on macOS with Xcode/Swift" >&2
  exit 1
fi

ORT_SRC=""
if command -v brew >/dev/null 2>&1; then
  ORT_PREFIX="$(brew --prefix onnxruntime 2>/dev/null || true)"
  if [[ -n "$ORT_PREFIX" && -f "$ORT_PREFIX/lib/libonnxruntime.dylib" ]]; then
    ORT_SRC="$ORT_PREFIX/lib/libonnxruntime.dylib"
  fi
fi
if [[ -z "$ORT_SRC" ]]; then
  echo "onnxruntime dylib not found — install: brew install onnxruntime" >&2
  exit 1
fi

chmod +x "$ROOT/scripts/build-macos-app-icon.sh"
"$ROOT/scripts/build-macos-app-icon.sh"

rm -rf "$WORK"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/Helpers" "$APP/Contents/Frameworks"

sed "s/__VERSION__/${VERSION}/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"
cp "$APP_BUILD" "$APP/Contents/MacOS/OpenPolySphere"
chmod +x "$APP/Contents/MacOS/OpenPolySphere"

cp "$ROOT/packaging/macos/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

cp "$BUILD/translator" "$APP/Contents/Resources/"
chmod +x "$APP/Contents/Resources/translator"

if command -v codesign >/dev/null 2>&1; then
  codesign --force --sign - "$APP/Contents/Resources/translator" 2>/dev/null || true
fi

if [[ -f "$BUILD/polysphere-translate" ]]; then
  cp "$BUILD/polysphere-translate" "$APP/Contents/Resources/"
  chmod +x "$APP/Contents/Resources/polysphere-translate"
fi

cp -R "$BUILD/PolySphereSpeech.app" "$APP/Contents/Resources/Helpers/"
cp -R "$ROOT/web" "$APP/Contents/Resources/"
cp "$ROOT/.env.example" "$APP/Contents/Resources/"
cp "$ORT_SRC" "$APP/Contents/Frameworks/libonnxruntime.dylib"

# Ad-hoc sign (local/Gatekeeper hint only — not notarized).
if command -v codesign >/dev/null 2>&1; then
  codesign --force --sign - "$APP/Contents/Resources/Helpers/PolySphereSpeech.app" 2>/dev/null || true
  codesign --force --sign - "$APP" 2>/dev/null || true
fi

rm -f "$ZIP" "$DMG"

(
  cd "$WORK"
  zip -r "$OUT_DIR/${ARTIFACT_BASE}.zip" OpenPolySphere.app
)

if command -v create-dmg >/dev/null 2>&1; then
  create-dmg \
    --volname "OpenPolySphere" \
    --volicon "$ROOT/packaging/macos/AppIcon.icns" \
    --window-pos 200 120 \
    --window-size 660 400 \
    --icon-size 128 \
    --icon "OpenPolySphere.app" 180 185 \
    --hide-extension "OpenPolySphere.app" \
    --app-drop-link 480 185 \
    --no-internet-enable \
    "$DMG" \
    "$WORK" >/dev/null
else
  DMG_STAGING="$OUT_DIR/.dmg-${ARTIFACT_BASE}"
  rm -rf "$DMG_STAGING"
  mkdir -p "$DMG_STAGING"
  cp -R "$APP" "$DMG_STAGING/"
  ln -s /Applications "$DMG_STAGING/Applications"
  hdiutil create -volname "OpenPolySphere" -srcfolder "$DMG_STAGING" -ov -format UDZO "$DMG" >/dev/null
  rm -rf "$DMG_STAGING"
fi

rm -rf "$WORK"

echo "Created $DMG ($(file "$BUILD/translator" | sed 's/.*: //'))"
echo "Created $ZIP"
echo "Install: open the .dmg, drag OpenPolySphere to Applications, launch from Launchpad."
echo "First run: /Applications/OpenPolySphere.app/Contents/MacOS/OpenPolySphere setup"
echo "Also: brew install espeak-ng"
if [[ "$ARCH" == "arm64" ]]; then
  echo "Build: Apple Silicon (M1/M2/M3). Intel Macs → macos-x64."
elif [[ "$ARCH" == "x86_64" ]]; then
  echo "Build: Intel Mac. Apple Silicon → macos-arm64."
fi
