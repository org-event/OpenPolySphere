#!/usr/bin/env bash
# Build OpenPolySphere.app from a release `cargo build --release -p translator`.
#
# Usage: ./scripts/package-macos-app.sh <version> [output-dir]
# Example: ./scripts/package-macos-app.sh 0.4.0 dist
#
# Produces: dist/openpolysphere-<version>-macos-<arch>.zip  (contains OpenPolySphere.app only)

set -euo pipefail

VERSION="${1:?usage: package-macos-app.sh <version> [output-dir]}"
OUT_DIR="${2:-dist}"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH_LABEL="x64" ;;
  *) ARCH_LABEL="$ARCH" ;;
esac
ZIP_BASE="openpolysphere-${VERSION}-macos-${ARCH_LABEL}"
WORK="$OUT_DIR/.pack-${ZIP_BASE}"
APP="$WORK/OpenPolySphere.app"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD="$ROOT/target/release"

cd "$ROOT"

if [[ ! -x "$BUILD/translator" ]]; then
  echo "missing $BUILD/translator — run: cargo build --release -p translator" >&2
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

rm -rf "$WORK"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/Helpers" "$APP/Contents/Frameworks"

sed "s/__VERSION__/${VERSION}/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"
cp "$ROOT/packaging/macos/OpenPolySphere" "$APP/Contents/MacOS/OpenPolySphere"
chmod +x "$APP/Contents/MacOS/OpenPolySphere"

cp "$BUILD/translator" "$APP/Contents/Resources/"
chmod +x "$APP/Contents/Resources/translator"

if [[ -f "$BUILD/polysphere-translate" ]]; then
  cp "$BUILD/polysphere-translate" "$APP/Contents/Resources/"
  chmod +x "$APP/Contents/Resources/polysphere-translate"
fi

cp -R "$BUILD/PolySphereSpeech.app" "$APP/Contents/Resources/Helpers/"
cp -R "$ROOT/web" "$APP/Contents/Resources/"
cp "$ROOT/.env.example" "$APP/Contents/Resources/"

cp "$ORT_SRC" "$APP/Contents/Frameworks/libonnxruntime.dylib"

rm -f "$OUT_DIR/${ZIP_BASE}.zip"
mkdir -p "$OUT_DIR"
(
  cd "$WORK"
  zip -r "../${ZIP_BASE}.zip" OpenPolySphere.app
)
rm -rf "$WORK"

echo "Created $OUT_DIR/${ZIP_BASE}.zip ($(file "$BUILD/translator" | sed 's/.*: //'))"
echo "Install: unzip, drag OpenPolySphere.app to Applications, double-click."
echo "First run downloads models — also install: brew install espeak-ng"
if [[ "$ARCH" == "arm64" ]]; then
  echo "This build is for Apple Silicon (M1/M2/M3). Intel Macs need the macos-x64 artifact."
elif [[ "$ARCH" == "x86_64" ]]; then
  echo "This build is for Intel Macs. Apple Silicon needs the macos-arm64 artifact."
fi
