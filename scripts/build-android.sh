#!/usr/bin/env bash
# Builds libosmo_bridge.so into android/app/src/main/jniLibs.
# Used by Gradle's buildRust task and by CI/CLI builds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NDK="${ANDROID_NDK_HOME:-$ROOT/.tools/android-ndk}"
if [ ! -d "$NDK" ]; then
  echo "Android NDK not found. Set ANDROID_NDK_HOME or install NDK 29 to .tools/android-ndk." >&2
  exit 1
fi

CARGO_NDK="$ROOT/.tools/bin/cargo-ndk"
if [ -x "$CARGO_NDK" ]; then
  export PATH="$(dirname "$CARGO_NDK"):$PATH"
elif ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "cargo-ndk not found. Run: cargo install cargo-ndk" >&2
  exit 1
fi

export ANDROID_NDK_HOME="$NDK"
cd "$ROOT"
cargo ndk \
  -t "${ABI:-arm64-v8a}" \
  -P "${API_LEVEL:-29}" \
  -o "$ROOT/android/app/src/main/jniLibs" \
  build --release -p android-bridge
