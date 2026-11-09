#!/usr/bin/env bash
# ==============================================================================
# Compila o núcleo Rust para iOS, gera os bindings Swift (UniFFI) e monta o
# XCFramework em ios/Frameworks. Gerado via jirau — pode ser usado sem a CLI.
#
# Uso: scripts/build-ios-ffi.sh [debug|release]
#   IOS_TARGETS="aarch64-apple-ios-sim aarch64-apple-ios"  sobrescreve os alvos
# ==============================================================================
set -euo pipefail

PROFILE="${1:-debug}"
CRATE="all-platforms-showcase-ffi"
LIB="all_platforms_showcase_ffi"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

case "$PROFILE" in
  debug)
    RELEASE_FLAG=""
    DEFAULT_TARGETS="aarch64-apple-ios-sim"
    ;;
  release)
    RELEASE_FLAG="--release"
    DEFAULT_TARGETS="aarch64-apple-ios aarch64-apple-ios-sim"
    ;;
  *)
    echo "Perfil inválido: $PROFILE (use 'debug' ou 'release')" >&2
    exit 2
    ;;
esac

read -r -a TARGETS <<< "${IOS_TARGETS:-$DEFAULT_TARGETS}"
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}"

if command -v rustup >/dev/null 2>&1; then
  INSTALLED="$(rustup target list --installed)"
  for t in "${TARGETS[@]}"; do
    if ! grep -qx "$t" <<< "$INSTALLED"; then
      echo "➜ Instalando target Rust $t..."
      rustup target add "$t"
    fi
  done
fi

# 1. Biblioteca estática por alvo iOS
for t in "${TARGETS[@]}"; do
  echo "➜ cargo build ($t, $PROFILE)"
  cargo build -p "$CRATE" --lib --target "$t" $RELEASE_FLAG
done

# 2. Bindings Swift: library mode lê os metadados UniFFI de um build de host
#    em debug (o perfil release usa strip, que remove esses metadados).
echo "➜ Gerando bindings Swift via UniFFI"
cargo build -p "$CRATE" --lib
WORK="target/jirau-ios/$PROFILE"
rm -rf "$WORK"
mkdir -p "$WORK/swift" "$WORK/headers"
cargo run -q -p "$CRATE" --bin uniffi-bindgen -- generate   --library "target/debug/lib${LIB}.dylib"   --language swift   --out-dir "$WORK/swift"

cp "$WORK"/swift/*.h "$WORK/headers/"
cat "$WORK"/swift/*.modulemap > "$WORK/headers/module.modulemap"

mkdir -p ios/Generated ios/Frameworks
find ios/Generated -name '*.swift' -delete
cp "$WORK"/swift/*.swift ios/Generated/

# 3. XCFramework (uma fatia por plataforma)
XCF="ios/Frameworks/${LIB}.xcframework"
rm -rf "$XCF"
ARGS=()
for t in "${TARGETS[@]}"; do
  ARGS+=(-library "target/$t/$PROFILE/lib${LIB}.a" -headers "$WORK/headers")
done
xcodebuild -create-xcframework "${ARGS[@]}" -output "$XCF" >/dev/null

echo "✅ Núcleo Rust pronto para iOS: $XCF + ios/Generated/"
