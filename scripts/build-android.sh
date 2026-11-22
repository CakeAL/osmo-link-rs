#!/usr/bin/env bash
# Builds the native library and generates its Kotlin bindings.
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
abi="${ABI:-arm64-v8a}"
jni_root="$ROOT/android/app/src/main/jniLibs"
bindings="${1:-$ROOT/android/app/build/generated/osmo-bindings}"
cargo ndk \
  -t "$abi" \
  -P "${API_LEVEL:-29}" \
  -o "$jni_root" \
  build --locked --release -p mobile-bridge

# Remove the obsolete library from upgrades of existing checkouts.
rm -f "$jni_root/$abi/libosmo_bridge.so"
cargo run --locked -p mobile-bridge --features bindgen --bin osmo-bindgen -- \
  generate --library "$jni_root/$abi/libosmo_mobile.so" --language kotlin --no-format --out-dir "$bindings"
