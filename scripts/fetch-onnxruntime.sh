#!/usr/bin/env bash
# Download ONNX Runtime next to the repo (CI uses the same layout under ort/).
set -euo pipefail
cd "$(dirname "$0")/.."

ORT_VERSION="${ORT_VERSION:-1.20.1}"

case "$(uname -s)" in
  Linux)
    ORT_DIR="ort/onnxruntime-linux-x64-${ORT_VERSION}"
    LIB="$ORT_DIR/lib/libonnxruntime.so"
    if [[ -f "$LIB" ]]; then
      echo "[ok] $LIB"
    else
      mkdir -p ort
      TGZ="onnxruntime-linux-x64-${ORT_VERSION}.tgz"
      echo "[..] Downloading $TGZ..."
      curl -fsSL -o ort.tgz \
        "https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${TGZ}"
      tar -xzf ort.tgz -C ort
      rm -f ort.tgz
      echo "[ok] extracted to $ORT_DIR"
    fi
    echo ""
    echo "Add to your shell (or .env):"
    echo "  export ORT_DYLIB_PATH=\"$PWD/$LIB\""
    echo "  export LD_LIBRARY_PATH=\"$PWD/$ORT_DIR/lib:\$LD_LIBRARY_PATH\""
    ;;
  Darwin)
    if [[ -f /opt/homebrew/lib/libonnxruntime.dylib ]]; then
      echo "[ok] /opt/homebrew/lib/libonnxruntime.dylib (brew)"
      echo "  export ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib"
    elif [[ -f /usr/local/lib/libonnxruntime.dylib ]]; then
      echo "[ok] /usr/local/lib/libonnxruntime.dylib"
      echo "  export ORT_DYLIB_PATH=/usr/local/lib/libonnxruntime.dylib"
    else
      echo "[!] brew install onnxruntime   # or set ORT_DYLIB_PATH manually"
      exit 1
    fi
    ;;
  MINGW* | MSYS* | CYGWIN*)
    ORT_DIR="ort/onnxruntime-win-x64-${ORT_VERSION}"
    DLL="$ORT_DIR/lib/onnxruntime.dll"
    if [[ -f "$DLL" ]]; then
      echo "[ok] $DLL"
    else
      mkdir -p ort
      ZIP="onnxruntime-win-x64-${ORT_VERSION}.zip"
      echo "[..] Downloading $ZIP..."
      curl -fsSL -o ort.zip \
        "https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ZIP}"
      unzip -q ort.zip -d ort
      rm -f ort.zip
      echo "[ok] extracted to $ORT_DIR"
    fi
    echo "Place onnxruntime.dll next to translator.exe or set ORT_DYLIB_PATH"
    ;;
  *)
    echo "[!] Unsupported OS for fetch-onnxruntime.sh"
    exit 1
    ;;
esac
