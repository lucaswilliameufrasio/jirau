# 🤖 Guia de Configuração do Ambiente Android (Debian / Linux) — Secure Vault

Este guia descreve os passos necessários para configurar o ambiente de desenvolvimento Android completo para o **Secure Vault**, incluindo a compilação do núcleo nativo em **Rust** via `cargo-ndk`.

---

## 1. Instalação do JDK e Dependências do Sistema

No Debian GNU/Linux (ou derivados como Ubuntu/Mint), instale o OpenJDK 17 ou 21 e as bibliotecas essenciais:

```bash
sudo apt update
sudo apt install -y openjdk-21-jdk curl git unzip libglu1-mesa
```

### Aceleração de Hardware para o Emulador (KVM)

Para rodar emuladores Android com aceleração de hardware:

```bash
sudo apt install -y qemu-kvm
sudo usermod -aG kvm $USER
```
*(Reinicie a sessão ou dê logout/login para aplicar o novo grupo).*

---

## 2. Servidores Headless (Debian 13 CLI / Sem Interface Gráfica)

Em servidores ou ambientes headless (sem ambiente gráfico de desktop), **não é necessário instalar o Android Studio**. O ambiente é 100% gerenciado via linha de comando com `mise`, `sdkmanager` e `cargo-ndk`:

### 2.1. Instalar o Android SDK Command-line Tools

```bash
mkdir -p ~/Android/Sdk/cmdline-tools
curl -L -o /tmp/cmdline-tools.zip https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
python3 -m zipfile -e /tmp/cmdline-tools.zip /tmp/cmdline-extracted
mv /tmp/cmdline-extracted/cmdline-tools ~/Android/Sdk/cmdline-tools/latest
chmod +x ~/Android/Sdk/cmdline-tools/latest/bin/*
rm -rf /tmp/cmdline-tools.zip /tmp/cmdline-extracted
```

### 2.2. Aceitar Licenças e Instalar Componentes (SDK, NDK, Platform-tools)

```bash
# Aceita todas as licenças do SDK
yes | ~/Android/Sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/Android/Sdk --licenses

# Instala os pacotes necessários
~/Android/Sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/Android/Sdk \
  "platform-tools" \
  "platforms;android-35" \
  "build-tools;35.0.0" \
  "ndk;27.2.12479018" \
  "cmake;3.22.1"
```

---

## 3. Workstations com Interface Gráfica (Android Studio)

Se você estiver em uma máquina local com desktop (GNOME, KDE, etc.) e quiser utilizar o IDE:

### Opção A: Flatpak (Recomendado para isolamento)

```bash
flatpak install flathub com.google.AndroidStudio
flatpak run com.google.AndroidStudio
```

### Opção B: Tarball Oficial

1. Baixe o `.tar.gz` estável em [developer.android.com/studio](https://developer.android.com/studio).
2. Extraia para `/opt`:
   ```bash
   sudo tar -xvzf android-studio-*.tar.gz -C /opt/
   ```
3. Inicie o instalador:
   ```bash
   /opt/android-studio/bin/studio.sh
   ```

---

## 4. Componentes Obrigatórios do SDK & NDK

Se configurou via interface do Android Studio:
**Settings** $\rightarrow$ **Languages & Frameworks** $\rightarrow$ **Android SDK** $\rightarrow$ aba **SDK Tools**:
- [x] **Android SDK Command-line Tools (latest)**
- [x] **Android SDK Platform-Tools** (`adb`)
- [x] **NDK (Side by side)** (`27.2.12479018` ou `26.3`)
- [x] **CMake (3.22+)**

Por padrão no Linux, o SDK será instalado em `~/Android/Sdk`.

---

## 5. Variáveis de Ambiente

Configure as variáveis para que ferramentas de linha de comando (`adb`, `cargo-ndk`, `gradlew`) encontrem o SDK.

### Bash (`~/.bashrc`) ou Zsh (`~/.zshrc`)

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$HOME/.cargo/bin:$PATH"
```

### Fish Shell (`~/.config/fish/config.fish`)

```fish
set -gx ANDROID_HOME "$HOME/Android/Sdk"
set -gx ANDROID_SDK_ROOT "$ANDROID_HOME"
set -gx ANDROID_NDK_HOME "$ANDROID_HOME/ndk/27.2.12479018"
fish_add_path "$ANDROID_HOME/cmdline-tools/latest/bin" "$ANDROID_HOME/platform-tools" "$HOME/.cargo/bin"
```

Após editar, recarregue seu shell:
```bash
source ~/.bashrc # ou source ~/.zshrc
```

---

## 6. Toolchain Rust para Android (`cargo-ndk` e Targets)

Como o Secure Vault possui núcleo compartilhado em Rust, o compilador precisa dos alvos Android instalados:

```bash
# Adicionar os alvos de compilação cruzada do Rust
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android

# Instalar o utilitário cargo-ndk
cargo install cargo-ndk
```

### Script Automatizado

O repositório disponibiliza um script que verifica e configura essas dependências automaticamente:

```bash
./scripts/setup-android-env.sh
```

---

## 7. Verificação do Ambiente

Para validar se tudo está configurado corretamente:

1. Execute o diagnóstico do jirau:
   ```bash
   jirau doctor
   ```
2. Teste o `adb`:
   ```bash
   adb devices
   ```
3. Teste o `cargo-ndk`:
   ```bash
   cargo ndk --version
   ```
4. Teste os targets do compilador Rust:
   ```bash
   rustup target list --installed
   ```
5. Realize o primeiro build do Gradle:
   ```bash
   cd android && ./gradlew assembleDebug
   ```
