#!/usr/bin/env bash
# Linux release: portable zip + .deb + .rpm (via nfpm).
#
# Usage: ./scripts/package-linux.sh <version> [output-dir]
# Requires: built target/release/translator, ORT at ort/onnxruntime-linux-x64-*/

set -euo pipefail

VERSION="${1:?usage: package-linux.sh <version> [output-dir]}"
OUT_DIR="${2:-dist}"
ORT_VERSION="${ORT_VERSION:-1.20.1}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD="$ROOT/target/release"
ORT_DIR="$ROOT/ort/onnxruntime-linux-x64-${ORT_VERSION}"
PACK_ROOT="$ROOT/packaging/linux/.pack-root"
ZIP_NAME="openpolysphere-${VERSION}-linux-x64"
PORTABLE="$OUT_DIR/${ZIP_NAME}"

cd "$ROOT"

if [[ ! -x "$BUILD/translator" ]]; then
  echo "missing $BUILD/translator" >&2
  exit 1
fi
if [[ ! -f "$ORT_DIR/lib/libonnxruntime.so" ]]; then
  echo "missing $ORT_DIR/lib/libonnxruntime.so — run fetch-ort or release ORT step" >&2
  exit 1
fi

# FHS tree for nfpm
rm -rf "$PACK_ROOT"
mkdir -p "$PACK_ROOT/usr/lib/openpolysphere/bin" "$PACK_ROOT/usr/lib/openpolysphere/lib"
mkdir -p "$PACK_ROOT/usr/share/openpolysphere"

cp "$BUILD/translator" "$PACK_ROOT/usr/lib/openpolysphere/bin/"
chmod +x "$PACK_ROOT/usr/lib/openpolysphere/bin/translator"
cp "$ORT_DIR/lib/libonnxruntime.so" "$PACK_ROOT/usr/lib/openpolysphere/lib/"
cp -R "$ROOT/web" "$PACK_ROOT/usr/share/openpolysphere/"

# Portable zip (flat layout, same as before)
rm -rf "$PORTABLE"
mkdir -p "$PORTABLE"
cp "$BUILD/translator" "$PORTABLE/"
chmod +x "$PORTABLE/translator"
cp "$ORT_DIR/lib/libonnxruntime.so" "$PORTABLE/"
cp -R "$ROOT/web" "$PORTABLE/web"
cp "$ROOT/.env.example" "$PORTABLE/"
[[ -f "$ROOT/README.md" ]] && cp "$ROOT/README.md" "$PORTABLE/"
[[ -f "$ROOT/README.ru.md" ]] && cp "$ROOT/README.ru.md" "$PORTABLE/"
cat > "$PORTABLE/LINUX.txt" <<EOF
OpenPolySphere ${VERSION} — Linux x64

Portable zip:
  1. unzip, cd into folder
  2. export ORT_DYLIB_PATH=\$PWD/libonnxruntime.so
  3. export LD_LIBRARY_PATH=\$PWD:\$LD_LIBRARY_PATH
  4. ./translator setup
  5. ./translator  →  http://127.0.0.1:5050

Package install (recommended):
  sudo apt install ./openpolysphere_${VERSION}_amd64.deb   # Debian/Ubuntu
  sudo dnf install ./openpolysphere-${VERSION}-1.x86_64.rpm   # Fedora
  openpolysphere setup
  openpolysphere

Also: sudo apt install espeak-ng  (if not pulled in as dependency)
EOF

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR/${ZIP_NAME}.zip"
(
  cd "$OUT_DIR"
  zip -r "${ZIP_NAME}.zip" "$(basename "$PORTABLE")"
)
rm -rf "$PORTABLE"

# nfpm — .deb and .rpm
NFPM_VER="2.41.1"
NFPM_BIN="$ROOT/packaging/linux/.nfpm"
if [[ ! -x "$NFPM_BIN" ]]; then
  curl -fsSL -o /tmp/nfpm.tgz \
    "https://github.com/goreleaser/nfpm/releases/download/v${NFPM_VER}/nfpm_${NFPM_VER}_Linux_x86_64.tar.gz"
  tar -xzf /tmp/nfpm.tgz -C /tmp nfpm
  mv /tmp/nfpm "$NFPM_BIN"
  chmod +x "$NFPM_BIN"
fi

rm -f "$OUT_DIR"/openpolysphere_"${VERSION}"_amd64.deb
rm -f "$OUT_DIR"/openpolysphere-"${VERSION}"-*.x86_64.rpm

NFPM_CFG="$ROOT/packaging/linux/.nfpm.yaml"
sed "s/^version:.*/version: \"${VERSION}\"/" "$ROOT/packaging/linux/nfpm.yaml" > "$NFPM_CFG"

"$NFPM_BIN" pkg -f "$NFPM_CFG" -t "$OUT_DIR" --packager deb
"$NFPM_BIN" pkg -f "$NFPM_CFG" -t "$OUT_DIR" --packager rpm

echo "Created:"
echo "  $OUT_DIR/${ZIP_NAME}.zip"
echo "  $OUT_DIR/openpolysphere_${VERSION}_amd64.deb"
ls -1 "$OUT_DIR"/openpolysphere-"${VERSION}"-*.x86_64.rpm 2>/dev/null || true
