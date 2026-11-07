#!/bin/sh
# Builds the Rust library and generates the Swift bindings from it (UniFFI).
# Run from the project folder:  scripts/build-ffi.sh
set -eu
cd "$(dirname "$0")/.."

# Same minimum version as the Swift package (macOS 14), so the linker does not complain
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release -p snapious-ffi
LIB=target/release/libsnapious_ffi.dylib

# Swift source file
cargo run -q --release -p snapious-ffi --bin uniffi-bindgen-swift -- \
  --swift-sources "$LIB" app/Sources/SnapiousFFI
# C header and module map for the C target in the Swift package
cargo run -q --release -p snapious-ffi --bin uniffi-bindgen-swift -- \
  --headers --modulemap --module-name snapious_ffiFFI --modulemap-filename module.modulemap \
  "$LIB" app/Sources/CSnapiousFFI/include

echo "Done: bindings in app/Sources/SnapiousFFI and app/Sources/CSnapiousFFI/include"
