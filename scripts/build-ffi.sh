#!/bin/sh
# Baut die Rust-Bibliothek und erzeugt daraus die Swift-Bindings (UniFFI).
# Aufruf aus dem Projektordner:  scripts/build-ffi.sh
set -eu
cd "$(dirname "$0")/.."

# Gleiche Mindestversion wie das Swift-Paket (macOS 14), damit der Linker nicht meckert
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release -p snapious-ffi
LIB=target/release/libsnapious_ffi.dylib

# Swift-Quelldatei
cargo run -q --release -p snapious-ffi --bin uniffi-bindgen-swift -- \
  --swift-sources "$LIB" app/Sources/SnapiousFFI
# C-Header und Modulbeschreibung für den C-Zieltyp im Swift-Paket
cargo run -q --release -p snapious-ffi --bin uniffi-bindgen-swift -- \
  --headers --modulemap --module-name snapious_ffiFFI --modulemap-filename module.modulemap \
  "$LIB" app/Sources/CSnapiousFFI/include

echo "Fertig: Bindings in app/Sources/SnapiousFFI und app/Sources/CSnapiousFFI/include"
