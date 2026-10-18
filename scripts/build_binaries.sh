#!/usr/bin/env bash
# Build the binaries that can be produced on this host.
#
# macOS hosts build:
#   - a universal (arm64 + x86_64) macOS binary
#   - a Windows x86_64 binary when mingw-w64 is installed
# Linux hosts build a native Linux binary, but a macOS host cannot cross-build
# Linux without a target sysroot (ALSA/X11/libudev); use the GitHub Actions
# release workflow for Linux/Windows-native artifacts.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p dist

HOST=$(rustc -vV | awk '/^host:/{print $2}')
echo "host: $HOST"

case "$HOST" in
  *apple-darwin*)
    cargo build --release
    rustup target add x86_64-apple-darwin >/dev/null 2>&1 || true
    cargo build --release --target x86_64-apple-darwin
    if [ "$HOST" = "aarch64-apple-darwin" ]; then
      lipo -create target/release/digger target/x86_64-apple-darwin/release/digger \
        -output dist/digger-macos-universal
    else
      cp target/release/digger dist/digger-macos-x86_64
    fi
    ;;
  *linux*)
    cargo build --release
    cp target/release/digger "dist/digger-linux-$HOST"
    ;;
  *windows*)
    cargo build --release
    cp target/release/digger.exe dist/digger-windows-x86_64.exe
    ;;
  *)
    echo "unsupported host: $HOST" >&2
    ;;
esac

# Windows cross-build is practical with mingw-w64 (no Windows-only SDK needed).
if [ "$HOST" != *windows* ] && command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
  echo "building Windows x86_64 via mingw-w64"
  rustup target add x86_64-pc-windows-gnu >/dev/null 2>&1 || true
  cargo build --release --target x86_64-pc-windows-gnu
  cp target/x86_64-pc-windows-gnu/release/digger.exe dist/digger-windows-x86_64.exe
fi

if [ "$HOST" != *linux* ]; then
  echo
  echo "NOTE: Linux binaries need a Linux host/container (ALSA, libudev, X11/Wayland"
  echo "      build dependencies). Use .github/workflows/release.yml or a Linux VM."
fi

ls -lh dist
