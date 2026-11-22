#!/bin/bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
export CARGO_TARGET_DIR="$DERIVED_FILE_DIR/osmo-rust"
export IPHONEOS_DEPLOYMENT_TARGET
workspace="$(cd "$SRCROOT/.." && pwd)"

profile=debug
cargo_options=()
if [[ "$CONFIGURATION" != Debug ]]; then
    profile=release
    cargo_options+=(--release)
fi

libraries=()
for arch in $ARCHS; do
    case "$PLATFORM_NAME:$arch" in
        iphoneos:arm64) target=aarch64-apple-ios ;;
        iphonesimulator:arm64) target=aarch64-apple-ios-sim ;;
        iphonesimulator:x86_64) target=x86_64-apple-ios ;;
        *)
            echo "error: Unsupported Rust destination $PLATFORM_NAME:$arch"
            exit 1
            ;;
    esac
    if ! rustup target list --installed | /usr/bin/grep -Fx "$target" >/dev/null; then
        echo "error: Run rustup target add $target before building Osmo Link."
        exit 1
    fi
    cargo build --manifest-path "$workspace/Cargo.toml" --locked -p mobile-bridge \
        --target "$target" ${cargo_options[@]+"${cargo_options[@]}"}
    libraries+=("$CARGO_TARGET_DIR/$target/$profile/libosmo_mobile.a")
done

if [[ ${#libraries[@]} -eq 1 ]]; then
    cp "${libraries[0]}" "$SCRIPT_OUTPUT_FILE_0"
else
    xcrun lipo -create "${libraries[@]}" -output "$SCRIPT_OUTPUT_FILE_0"
fi

bindings="$DERIVED_FILE_DIR/OsmoMobile"
mkdir -p "$bindings"
# Read metadata from a single archive before lipo, regardless of the Xcode destination.
env -u SDKROOT -u IPHONEOS_DEPLOYMENT_TARGET -u MACOSX_DEPLOYMENT_TARGET \
    cargo run --manifest-path "$workspace/Cargo.toml" --locked \
    -p mobile-bridge --features bindgen --bin osmo-bindgen -- \
    generate --library "${libraries[0]}" --language swift --no-format --out-dir "$bindings"
mv "$bindings/OsmoMobileFFI.modulemap" "$bindings/module.modulemap"
