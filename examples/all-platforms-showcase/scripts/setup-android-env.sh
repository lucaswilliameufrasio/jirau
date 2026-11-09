#!/usr/bin/env bash
set -euo pipefail

# Garante que ~/.cargo/bin e ~/.local/bin estejam no PATH se existirem
if [ -d "$HOME/.cargo/bin" ] && [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi
if [ -d "$HOME/.local/bin" ] && [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    export PATH="$HOME/.local/bin:$PATH"
fi

echo "=========================================================="
echo "  🪵 all-platforms-showcase - Verificação & Setup do Ambiente Android"
echo "=========================================================="

# 1. Rust Targets para Android
echo ""
echo "==> [1/4] Verificando e instalando targets Android no Rustup..."
if command -v rustup >/dev/null 2>&1; then
    rustup target add \
        aarch64-linux-android \
        armv7-linux-androideabi \
        x86_64-linux-android \
        i686-linux-android || echo "Aviso: Falha ao baixar componentes (verifique a conexão à internet)."
    echo "Targets Android do Rust verificados!"
else
    echo "AVISO: rustup não encontrado no PATH."
fi

# 2. cargo-ndk
echo ""
echo "==> [2/4] Verificando cargo-ndk..."
if command -v cargo >/dev/null 2>&1; then
    if ! command -v cargo-ndk >/dev/null 2>&1; then
        echo "Tentando instalar cargo-ndk via cargo..."
        cargo install cargo-ndk || echo "Aviso: Não foi possível instalar cargo-ndk agora (verifique a conexão à internet)."
    else
        echo "cargo-ndk já instalado: $(cargo-ndk --version)"
    fi
else
    echo "AVISO: cargo não encontrado no PATH."
fi

# 3. Java / JDK
echo ""
echo "==> [3/4] Verificando Java (JDK)..."
if command -v java >/dev/null 2>&1; then
    java -version
else
    echo "AVISO: JDK não encontrado. Instale com:"
    echo "       mise install   # JDK do .mise.toml do projeto"
fi

# 4. Android SDK & NDK
echo ""
echo "==> [4/4] Verificando variáveis de ambiente do Android SDK..."
if [ -n "${ANDROID_HOME:-}" ] || [ -n "${ANDROID_SDK_ROOT:-}" ]; then
    SDK_DIR="${ANDROID_HOME:-$ANDROID_SDK_ROOT}"
    echo "Android SDK detectado em: $SDK_DIR"
else
    DEFAULT_SDK="$HOME/Android/Sdk"
    if [ -d "$DEFAULT_SDK" ]; then
        echo "Android SDK localizado no diretório padrão: $DEFAULT_SDK"
        echo "Adicione ao seu shell (~/.bashrc, ~/.zshrc ou ~/.config/fish/config.fish):"
        echo "  export ANDROID_HOME=\"$DEFAULT_SDK\""
        echo "  export PATH=\"\$ANDROID_HOME/cmdline-tools/latest/bin:\$ANDROID_HOME/platform-tools:\$PATH\""
    else
        echo "Android SDK não encontrado em \$HOME/Android/Sdk."
        echo "Consulte docs/setup-android.md para instalar via CLI headless ou Android Studio."
    fi
fi

echo ""
echo "Setup concluído!"
