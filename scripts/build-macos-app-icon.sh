#!/usr/bin/env bash
# Build packaging/macos/AppIcon.icns from the tree SVG logo.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SVG="$ROOT/site/img/openpolysphere-tree-icon.svg"
ICONSET="$ROOT/packaging/macos/AppIcon.iconset"
ICNS="$ROOT/packaging/macos/AppIcon.icns"
WORK="$(mktemp -d)"

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

if [[ ! -f "$SVG" ]]; then
  echo "missing logo SVG: $SVG" >&2
  exit 1
fi

MASTER="$WORK/master.png"
if command -v rsvg-convert >/dev/null 2>&1; then
  rsvg-convert -w 1024 -h 1024 "$SVG" -o "$MASTER"
elif command -v qlmanage >/dev/null 2>&1; then
  qlmanage -t -s 1024 -o "$WORK" "$SVG" >/dev/null 2>&1
  mv "$WORK/$(basename "$SVG").png" "$MASTER"
else
  echo "install rsvg-convert (brew install librsvg) or use macOS qlmanage" >&2
  exit 1
fi

rm -rf "$ICONSET"
mkdir -p "$ICONSET"

add_icon() {
  local size="$1"
  local name="$2"
  sips -z "$size" "$size" "$MASTER" --out "$ICONSET/$name" >/dev/null
}

add_icon 16 icon_16x16.png
add_icon 32 icon_16x16@2x.png
add_icon 32 icon_32x32.png
add_icon 64 icon_32x32@2x.png
add_icon 128 icon_128x128.png
add_icon 256 icon_128x128@2x.png
add_icon 256 icon_256x256.png
add_icon 512 icon_256x256@2x.png
add_icon 512 icon_512x512.png
add_icon 1024 icon_512x512@2x.png

iconutil -c icns "$ICONSET" -o "$ICNS"
echo "Wrote $ICNS"
