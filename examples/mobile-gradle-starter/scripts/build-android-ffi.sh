#!/usr/bin/env bash
# ==============================================================================
# Compila o núcleo Rust para Android (cargo-ndk → jniLibs) e gera os bindings
# Kotlin (UniFFI). Gerado via jirau — o Gradle chama este script no preBuild,
# então funciona também pelo Android Studio, sem a CLI.
#
# Uso: scripts/build-android-ffi.sh [debug|release] [ABI...]
#   ABIs padrão: arm64-v8a x86_64 (ou a variável ANDROID_ABIS)
# ==============================================================================
set -euo pipefail

PROFILE="${1:-debug}"
shift || true
CRATE="mobile-gradle-starter-ffi"
LIB="mobile_gradle_starter_ffi"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

case "$PROFILE" in
  debug) RELEASE_FLAG="" ;;
  release) RELEASE_FLAG="--release" ;;
  *)
    echo "Perfil inválido: $PROFILE (use 'debug' ou 'release')" >&2
    exit 2
    ;;
esac

if [ "$#" -gt 0 ]; then
  ABIS=("$@")
else
  read -r -a ABIS <<< "${ANDROID_ABIS:-arm64-v8a x86_64}"
fi

# Garante ~/.cargo/bin no PATH (o Gradle/Android Studio nem sempre herda o shell).
if [ -d "$HOME/.cargo/bin" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

# SDK/NDK: respeita as variáveis do usuário e cai para os caminhos padrão.
if [ -z "${ANDROID_HOME:-}" ]; then
  for d in "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk" "$HOME/Library/Android/sdk"; do
    if [ -n "$d" ] && [ -d "$d" ]; then
      export ANDROID_HOME="$d"
      break
    fi
  done
fi
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME/ndk" ]; then
  NDK_VERSION="$(ls "$ANDROID_HOME/ndk" | sort -V | tail -1)"
  if [ -n "$NDK_VERSION" ]; then
    export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
  fi
fi
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "error: Android NDK não encontrado. Instale com: sdkmanager 'ndk;28.2.13676358'" >&2
  exit 1
fi

if ! cargo ndk --version >/dev/null 2>&1; then
  echo "➜ Instalando cargo-ndk..."
  cargo install cargo-ndk --locked
fi

abi_to_target() {
  case "$1" in
    arm64-v8a) echo "aarch64-linux-android" ;;
    armeabi-v7a) echo "armv7-linux-androideabi" ;;
    x86_64) echo "x86_64-linux-android" ;;
    x86) echo "i686-linux-android" ;;
    *) echo "error: ABI desconhecida: $1" >&2; exit 2 ;;
  esac
}

NDK_ARGS=()
if command -v rustup >/dev/null 2>&1; then
  INSTALLED="$(rustup target list --installed)"
fi
for abi in "${ABIS[@]}"; do
  target="$(abi_to_target "$abi")"
  if [ -n "${INSTALLED:-}" ] && ! grep -qx "$target" <<< "$INSTALLED"; then
    echo "➜ Instalando target Rust $target..."
    rustup target add "$target"
  fi
  NDK_ARGS+=(-t "$abi")
done

# 1. Biblioteca dinâmica (.so) por ABI em android/app/src/main/jniLibs/<abi>/
JNI_DIR="android/app/src/main/jniLibs"
echo "➜ cargo ndk (${ABIS[*]}, $PROFILE)"
cargo ndk "${NDK_ARGS[@]}" --platform 26 -o "$JNI_DIR" build -p "$CRATE" --lib $RELEASE_FLAG

# 2. Bindings Kotlin: library mode lê os metadados UniFFI de um build de host
#    em debug (o perfil release usa strip, que remove esses metadados).
case "$(uname -s)" in
  Darwin) HOST_LIB="target/debug/lib${LIB}.dylib" ;;
  MINGW*|MSYS*|CYGWIN*) HOST_LIB="target/debug/${LIB}.dll" ;;
  *) HOST_LIB="target/debug/lib${LIB}.so" ;;
esac
echo "➜ Gerando bindings Kotlin via UniFFI"
cargo build -p "$CRATE" --lib
OUT="android/app/build/generated/uniffi"
rm -rf "$OUT"
mkdir -p "$OUT"
cargo run -q -p "$CRATE" --bin uniffi-bindgen -- generate --library "$HOST_LIB" --language kotlin --no-format --out-dir "$OUT"

echo "✅ Núcleo Rust pronto para Android: $JNI_DIR + $OUT"
