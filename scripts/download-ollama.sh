#!/usr/bin/env bash
# Download the Ollama binary for the current build target.
# Usage: ./scripts/download-ollama.sh [target-triple]
#
# If no target is provided, detects the current platform.
# The binary is placed in src-tauri/binaries/ with the correct
# target-triple suffix required by Tauri's sidecar system.

set -euo pipefail

OLLAMA_VERSION="v0.6.2"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$SCRIPT_DIR/../src-tauri/binaries"
BINDIR="$(cd "$SCRIPT_DIR/../src-tauri/binaries" && pwd)"

# Detect or accept target triple
if [ -n "${1:-}" ]; then
  TARGET="$1"
else
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    Linux-aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
    Darwin-arm64)  TARGET="aarch64-apple-darwin" ;;
    MINGW*|MSYS*|CYGWIN*)
      TARGET="x86_64-pc-windows-msvc" ;;
    *)
      echo "Unsupported platform: $(uname -s)-$(uname -m)"
      exit 1
      ;;
  esac
fi

echo "Target: $TARGET"
echo "Ollama version: $OLLAMA_VERSION"

# Determine download URL
case "$TARGET" in
  x86_64-pc-windows-msvc)
    URL="https://github.com/ollama/ollama/releases/download/${OLLAMA_VERSION}/ollama-windows-amd64.zip"
    EXT=".exe"
    ;;
  x86_64-unknown-linux-gnu)
    URL="https://github.com/ollama/ollama/releases/download/${OLLAMA_VERSION}/ollama-linux-amd64.tgz"
    EXT=""
    ;;
  aarch64-unknown-linux-gnu)
    URL="https://github.com/ollama/ollama/releases/download/${OLLAMA_VERSION}/ollama-linux-arm64.tgz"
    EXT=""
    ;;
  x86_64-apple-darwin)
    URL="https://github.com/ollama/ollama/releases/download/${OLLAMA_VERSION}/Ollama-darwin.zip"
    EXT=""
    ;;
  aarch64-apple-darwin)
    URL="https://github.com/ollama/ollama/releases/download/${OLLAMA_VERSION}/Ollama-darwin.zip"
    EXT=""
    ;;
  *)
    echo "No Ollama binary available for target: $TARGET"
    exit 1
    ;;
esac

OUTFILE="${BINDIR}/ollama-${TARGET}${EXT}"

if [ -f "$OUTFILE" ]; then
  echo "Binary already exists: $OUTFILE"
  exit 0
fi

echo "Downloading from $URL ..."
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

case "$URL" in
  *.zip)
    curl -L -o "$TMPDIR/ollama.zip" "$URL"
    unzip -o "$TMPDIR/ollama.zip" -d "$TMPDIR/extracted"
    # Find the ollama binary
    FOUND="$(find "$TMPDIR/extracted" -name "ollama${EXT}" -type f | head -1)"
    if [ -z "$FOUND" ]; then
      echo "Could not find ollama binary in archive"
      exit 1
    fi
    cp "$FOUND" "$OUTFILE"
    ;;
  *.tgz)
    curl -L -o "$TMPDIR/ollama.tgz" "$URL"
    tar xzf "$TMPDIR/ollama.tgz" -C "$TMPDIR"
    FOUND="$(find "$TMPDIR" -name "ollama" -type f | head -1)"
    if [ -z "$FOUND" ]; then
      echo "Could not find ollama binary in archive"
      exit 1
    fi
    cp "$FOUND" "$OUTFILE"
    ;;
esac

chmod +x "$OUTFILE"
echo "Installed: $OUTFILE"
