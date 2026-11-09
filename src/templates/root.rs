use super::{TemplateContext, versions};

pub fn root_cargo_toml(ctx: &TemplateContext) -> String {
    let mut members = vec![format!("\"crates/{}-core\"", ctx.slug)];
    if ctx.platforms.contains(&"android".to_string()) || ctx.platforms.contains(&"ios".to_string())
    {
        members.push(format!("\"crates/{}-ffi\"", ctx.slug));
    }
    if ctx.platforms.contains(&"desktop".to_string()) {
        members.push(format!("\"crates/{}-desktop\"", ctx.slug));
    }
    let members_str = members
        .iter()
        .map(|m| format!("    {},", m))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"[workspace]
members = [
{members}
]
resolver = "3"

[profile.dev.package."*"]
opt-level = 2

[profile.release]
opt-level = 3
lto = "thin"
strip = true
"#,
        members = members_str
    )
}

pub fn root_gitignore() -> String {
    r#"# Rust
target/
**/*.rs.bk
*.swp

# Android
android/.gradle/
android/build/
android/*/build/
android/local.properties
android/.idea/
android/*.iml
android/*/*.iml
android/.cxx/
android/.kotlin/
android/app/build/generated/uniffi/
android/app/src/main/jniLibs/

# iOS
ios/.build/
ios/DerivedData/
ios/*.xcodeproj/
ios/Generated/
ios/Frameworks/

# OS
.DS_Store
Thumbs.db
"#
    .to_string()
}

pub fn root_mise_toml() -> String {
    format!(
        "[tools]\njava = \"{java}\"\nrust = \"{rust}\"\n",
        java = versions::JAVA_LTS,
        rust = versions::RUST
    )
}

pub fn testing_standard_md_pt_br(ctx: &TemplateContext) -> String {
    format!(
        r#"# Diretrizes de Testes — {name}

Este repositório adota a metodologia de testes rápidos em camada:

1. **Núcleo em Rust (`crates/{slug}-core`)**:
   - Testes unitários puros executados diretamente no host via `cargo test`.
   - Não dependem de emulador, simulador, Android NDK ou UniFFI.
   - Meta de execução: < 500 ms.

2. **Camada FFI (`crates/{slug}-ffi`)**:
   - Validação da integridade dos bindings gerados e serialização de tipos primitivos/complexos.

3. **Android (`android/app`)**:
   - Testes unitários com JUnit 5 + Mockito para ViewModels.
   - Previews no Jetpack Compose para validação visual estática.

4. **iOS (`ios/App`)**:
   - XCTest unitário para modelos `@Observable`.
   - Previews no SwiftUI Canvas.

5. **Desktop (`crates/{slug}-desktop`)**:
   - UI nativa renderizada em GPU com Rune (anomalyco/rune).
   - Reutiliza a crate `{slug}-core` diretamente em Rust, sem overhead de FFI.
   - Execução local: `cargo run --package {slug}-desktop`.

Execute no host:
```bash
cargo test --workspace
```
"#,
        name = ctx.name,
        slug = ctx.slug
    )
}

pub fn testing_standard_md_en(ctx: &TemplateContext) -> String {
    format!(
        r#"# Testing Guidelines — {name}

This repository follows a fast, layered testing methodology:

1. **Rust core (`crates/{slug}-core`)**:
   - Pure unit tests run directly on the host via `cargo test`.
   - They do not depend on an emulator, simulator, Android NDK or UniFFI.
   - Execution target: < 500 ms.

2. **FFI layer (`crates/{slug}-ffi`)**:
   - Validates the integrity of the generated bindings and the serialization of primitive/complex types.

3. **Android (`android/app`)**:
   - Unit tests with JUnit 5 + Mockito for ViewModels.
   - Jetpack Compose previews for static visual validation.

4. **iOS (`ios/App`)**:
   - XCTest unit tests for `@Observable` models.
   - SwiftUI Canvas previews.

5. **Desktop (`crates/{slug}-desktop`)**:
   - Native GPU-rendered UI with Rune (anomalyco/rune).
   - Reuses the `{slug}-core` crate directly in Rust, with no FFI overhead.
   - Run locally: `cargo run --package {slug}-desktop`.

Run on the host:
```bash
cargo test --workspace
```
"#,
        name = ctx.name,
        slug = ctx.slug
    )
}

pub fn jirau_toml(ctx: &TemplateContext) -> String {
    let platforms_str = ctx
        .platforms
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(", ");

    let plugins_str = ctx
        .plugins
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"[app]
name = "{name}"
package_id = "{package_id}"
version = "0.1.0"
platforms = [{platforms}]
storage = "{storage}"
android_build_system = "{android_build_system}"
ui_theme = "{ui_theme}"
plugins = [{plugins}]
template = "{app_template}"

[dev]
default_android_abi = "arm64-v8a"
default_ios_simulator = "iPhone 16"
use_sccache = true

[meta]
template_version = "{template_version}"
"#,
        name = ctx.name,
        package_id = ctx.package_id,
        platforms = platforms_str,
        storage = ctx.storage,
        android_build_system = ctx.android_build_system,
        ui_theme = ctx.ui_theme,
        plugins = plugins_str,
        app_template = ctx.app_template,
        template_version = ctx.template_version
    )
}

pub fn root_makefile(ctx: &TemplateContext) -> String {
    let has_android = ctx.platforms.contains(&"android".to_string());
    let has_ios = ctx.platforms.contains(&"ios".to_string());
    let has_desktop = ctx.platforms.contains(&"desktop".to_string());

    let mut help_entries = vec![
        "  make setup                  - Instala/verifica dependências de ambiente (mise, Java, Rust targets)",
        "  make doctor                 - Executa diagnóstico do ambiente de desenvolvimento",
        "  make devices                - Lista emuladores e dispositivos móveis conectados",
        "  make test                   - Executa todos os testes da workspace (Rust Core + integrações)",
        "  make test-core              - Executa testes unitários do núcleo Rust puro no host",
    ];
    if has_android {
        help_entries.push(
            "  make test-android           - Executa testes JVM do app Android (sem dispositivo)",
        );
        help_entries.push(
            "  make test-android-device    - Testes instrumentados (Rust real) no dispositivo conectado",
        );
    }
    if has_ios {
        help_entries.push(
            "  make test-ios               - Executa testes do target iOS via xcodebuild (macOS)",
        );
    }
    help_entries.push("  make check                  - Executa cargo check e cargo clippy");
    help_entries.push("  make format                 - Formata o código Rust (cargo fmt)");
    help_entries.push("  make build-core             - Compila as crates Rust da workspace");
    if has_android {
        help_entries.push(
            "  make android-ffi            - Compila o núcleo Rust para Android (.so + bindings Kotlin)",
        );
        help_entries.push("  make apk-debug              - Gera APK Android de Debug");
        help_entries.push("  make apk-release            - Gera APK Android de Release");
        help_entries.push("  make build-android          - Alias para apk-debug");
    }
    if has_ios {
        help_entries.push(
            "  make ios-ffi                - Compila o núcleo Rust para iOS (XCFramework + bindings Swift)",
        );
        help_entries.push(
            "  make ios-project            - Gera projeto Xcode via XcodeGen (ios/project.yml)",
        );
        help_entries
            .push("  make build-ios              - Compila o app iOS via xcodebuild (macOS)");
    }
    if has_desktop {
        help_entries.push(
            "  make build-desktop          - Compila o binário nativo Desktop em GPU via Rune",
        );
        help_entries
            .push("  make run-desktop            - Executa o app Desktop em desenvolvimento");
    }
    help_entries.push("  make clean                  - Limpa artefatos de compilação");

    // Cada linha de receita do Makefile precisa começar com TAB.
    let help_text = help_entries
        .iter()
        .map(|entry| format!("\t@echo \"{}\"", entry))
        .collect::<Vec<_>>()
        .join("\n");

    let mut targets = Vec::new();

    // setup body
    let mut setup_steps = Vec::new();
    if has_android {
        setup_steps.push(
            "\t@if [ -f scripts/setup-android-env.sh ]; then ./scripts/setup-android-env.sh; fi",
        );
        setup_steps.push("\t@$(RUSTUP) target add aarch64-linux-android x86_64-linux-android 2>/dev/null || true");
    }
    if has_ios {
        setup_steps.push(
            "\t@$(RUSTUP) target add aarch64-apple-ios aarch64-apple-ios-sim 2>/dev/null || true",
        );
    }
    let setup_body = if setup_steps.is_empty() {
        "\t@echo \"  Nenhuma etapa adicional requerida para o ambiente.\""
    } else {
        &setup_steps.join("\n")
    };

    if has_android {
        targets.push(
            r#"android-ffi:
	@$(RUN) ./scripts/build-android-ffi.sh debug

test-android:
	@cd android && $(RUN) ./gradlew :app:testDebugUnitTest

test-android-device:
	@cd android && $(RUN) ./gradlew :app:connectedDebugAndroidTest

apk-debug:
	@cd android && $(RUN) ./gradlew assembleDebug

apk-release:
	@cd android && $(RUN) ./gradlew assembleRelease

build-android: apk-debug
"#
            .to_string(),
        );
    }

    if has_ios {
        targets.push(format!(
            r#"ios-ffi:
	@$(RUN) ./scripts/build-ios-ffi.sh debug

ios-project: ios-ffi
	@cd ios && xcodegen generate

build-ios: ios-project
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		cd ios && xcodebuild -project {pascal_name}.xcodeproj -scheme {pascal_name} -configuration Debug -destination 'generic/platform=iOS Simulator' build CODE_SIGNING_ALLOWED=NO; \
	else \
		echo "ℹ️  Build iOS requer macOS com Xcode."; \
	fi

test-ios: ios-project
	@if [ "$$(uname -s)" != "Darwin" ]; then \
		echo "ℹ️  Testes iOS requerem macOS com Xcode."; \
	elif [ -z "$(IOS_DEVICE)" ]; then \
		echo "❌ Nenhum simulador iOS ligado. Inicie um com: xcrun simctl boot <UDID>"; exit 1; \
	else \
		cd ios && xcodebuild test -project {pascal_name}.xcodeproj -scheme {pascal_name} -destination 'platform=iOS Simulator,id=$(IOS_DEVICE)'; \
	fi
"#,
            pascal_name = ctx.pascal_name
        ));
    }

    if has_desktop {
        targets.push(format!(
            r#"build-desktop:
	@$(CARGO) build --package {slug}-desktop

run-desktop:
	@$(CARGO) run --package {slug}-desktop
"#,
            slug = ctx.slug
        ));
    }

    let extra_targets = targets.join("\n");

    format!(
        r#"# ==============================================================================
# 🪵 {name} - Makefile de Automação & Ferramentas
# Gerado via jirau (CLI de scaffold nativo multiplataforma)
# ==============================================================================

export ANDROID_HOME ?= $(HOME)/Android/Sdk
export PATH := $(HOME)/.local/bin:$(ANDROID_HOME)/platform-tools:$(ANDROID_HOME)/cmdline-tools/latest/bin:$(PATH)
IOS_DEVICE     ?= $(shell xcrun simctl list devices 2>/dev/null | grep -i booted | grep -oE '[0-9A-F-]{{36}}' | head -1)
MISE           := $(shell if [ -f "$$HOME/.local/bin/mise" ]; then echo "$$HOME/.local/bin/mise"; elif command -v mise >/dev/null 2>&1; then echo "mise"; else echo ""; fi)
RUN            := $(if $(MISE),$(MISE) exec --,)
CARGO          ?= $(shell command -v cargo 2>/dev/null || echo "$$HOME/.cargo/bin/cargo")
RUSTUP         ?= $(shell command -v rustup 2>/dev/null || echo "$$HOME/.cargo/bin/rustup")
ADB            ?= $(shell command -v adb 2>/dev/null || echo "$$HOME/Android/Sdk/platform-tools/adb")

.PHONY: all help setup doctor devices test test-core check format clean

all: help

help:
	@echo "🪵 {name} - Comandos disponíveis:"
	@echo ""
{help_text}

setup:
	@echo "==> Configurando ambiente do projeto..."
{setup_body}
	@echo "Setup concluído!"

doctor:
	@command -v jirau >/dev/null 2>&1 && jirau doctor || $(CARGO) run -- doctor

devices:
	@echo "==> Dispositivos detectados:"
	@command -v adb >/dev/null 2>&1 && adb devices -l || echo "adb não encontrado no PATH"
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		echo "--- iOS Simulators Booted ---"; \
		xcrun simctl list devices 2>/dev/null | grep -i booted || echo "Nenhum simulador iOS ativo"; \
	fi

test-core:
	@echo "==> Executando testes do núcleo Rust no host..."
	@$(CARGO) test --workspace

test: test-core

check:
	@echo "==> Executando cargo check e cargo clippy..."
	@$(CARGO) check --workspace
	@$(CARGO) clippy --all-targets -- -D warnings

format:
	@echo "==> Formatando código Rust..."
	@$(CARGO) fmt

build-core:
	@echo "==> Compilando crates Rust..."
	@$(CARGO) build --workspace

clean:
	@echo "==> Limpando artefatos de build..."
	@$(CARGO) clean
	@if [ -d android ]; then cd android && rm -rf .gradle build app/build; fi

{extra_targets}
"#,
        name = ctx.name,
        help_text = help_text,
        setup_body = setup_body,
        extra_targets = extra_targets
    )
}

pub fn setup_android_md_pt_br(ctx: &TemplateContext) -> String {
    format!(
        r#"# 🤖 Guia de Configuração do Ambiente Android (Debian / Linux) — {name}

Este guia descreve os passos necessários para configurar o ambiente de desenvolvimento Android completo para o **{name}**, incluindo a compilação do núcleo nativo em **Rust** via `cargo-ndk`.

---

## 1. Instalação do JDK e Dependências do Sistema

No Debian GNU/Linux (ou derivados como Ubuntu/Mint), instale o JDK {java} (LTS atual; o `.mise.toml` do projeto já o fixa) e as bibliotecas essenciais:

```bash
sudo apt update
sudo apt install -y curl git unzip libglu1-mesa
mise install   # JDK {java} e Rust {rust}, conforme .mise.toml
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
  "platforms;android-{sdk}" \
  "build-tools;{build_tools}" \
  "ndk;{ndk}"
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
- [x] **NDK (Side by side)** (`{ndk}`)

Por padrão no Linux, o SDK será instalado em `~/Android/Sdk`.

---

## 5. Variáveis de Ambiente

Configure as variáveis para que ferramentas de linha de comando (`adb`, `cargo-ndk`, `gradlew`) encontrem o SDK.

### Bash (`~/.bashrc`) ou Zsh (`~/.zshrc`)

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/{ndk}"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$HOME/.cargo/bin:$PATH"
```

### Fish Shell (`~/.config/fish/config.fish`)

```fish
set -gx ANDROID_HOME "$HOME/Android/Sdk"
set -gx ANDROID_SDK_ROOT "$ANDROID_HOME"
set -gx ANDROID_NDK_HOME "$ANDROID_HOME/ndk/{ndk}"
fish_add_path "$ANDROID_HOME/cmdline-tools/latest/bin" "$ANDROID_HOME/platform-tools" "$HOME/.cargo/bin"
```

Após editar, recarregue seu shell:
```bash
source ~/.bashrc # ou source ~/.zshrc
```

---

## 6. Toolchain Rust para Android (`cargo-ndk` e Targets)

Como o {name} possui núcleo compartilhado em Rust, o compilador precisa dos alvos Android instalados:

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
"#,
        name = ctx.name,
        java = versions::JAVA_LTS,
        rust = versions::RUST,
        sdk = versions::ANDROID_COMPILE_SDK,
        build_tools = versions::ANDROID_BUILD_TOOLS,
        ndk = versions::ANDROID_NDK,
    )
}

pub fn setup_android_md_en(ctx: &TemplateContext) -> String {
    format!(
        r#"# 🤖 Android Environment Setup Guide (Debian / Linux) — {name}

This guide describes the steps required to set up a complete Android development environment for **{name}**, including building the native **Rust** core via `cargo-ndk`.

---

## 1. Installing the JDK and System Dependencies

On Debian GNU/Linux (or derivatives such as Ubuntu/Mint), install JDK {java} (the current LTS; the project's `.mise.toml` already pins it) and the essential libraries:

```bash
sudo apt update
sudo apt install -y curl git unzip libglu1-mesa
mise install   # JDK {java} and Rust {rust}, as defined in .mise.toml
```

### Hardware Acceleration for the Emulator (KVM)

To run Android emulators with hardware acceleration:

```bash
sudo apt install -y qemu-kvm
sudo usermod -aG kvm $USER
```
*(Restart your session or log out/in to apply the new group.)*

---

## 2. Headless Servers (Debian 13 CLI / No Graphical Interface)

On servers or headless environments (no desktop environment), **you do not need to install Android Studio**. The environment is managed 100% from the command line with `mise`, `sdkmanager` and `cargo-ndk`:

### 2.1. Install the Android SDK Command-line Tools

```bash
mkdir -p ~/Android/Sdk/cmdline-tools
curl -L -o /tmp/cmdline-tools.zip https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
python3 -m zipfile -e /tmp/cmdline-tools.zip /tmp/cmdline-extracted
mv /tmp/cmdline-extracted/cmdline-tools ~/Android/Sdk/cmdline-tools/latest
chmod +x ~/Android/Sdk/cmdline-tools/latest/bin/*
rm -rf /tmp/cmdline-tools.zip /tmp/cmdline-extracted
```

### 2.2. Accept Licenses and Install Components (SDK, NDK, Platform-tools)

```bash
# Accept all SDK licenses
yes | ~/Android/Sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/Android/Sdk --licenses

# Install the required packages
~/Android/Sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/Android/Sdk \
  "platform-tools" \
  "platforms;android-{sdk}" \
  "build-tools;{build_tools}" \
  "ndk;{ndk}"
```

---

## 3. Workstations with a Graphical Interface (Android Studio)

If you are on a local machine with a desktop (GNOME, KDE, etc.) and want to use the IDE:

### Option A: Flatpak (Recommended for isolation)

```bash
flatpak install flathub com.google.AndroidStudio
flatpak run com.google.AndroidStudio
```

### Option B: Official Tarball

1. Download the stable `.tar.gz` from [developer.android.com/studio](https://developer.android.com/studio).
2. Extract it to `/opt`:
   ```bash
   sudo tar -xvzf android-studio-*.tar.gz -C /opt/
   ```
3. Start the installer:
   ```bash
   /opt/android-studio/bin/studio.sh
   ```

---

## 4. Required SDK & NDK Components

If you set things up through the Android Studio interface:
**Settings** $\rightarrow$ **Languages & Frameworks** $\rightarrow$ **Android SDK** $\rightarrow$ **SDK Tools** tab:
- [x] **Android SDK Command-line Tools (latest)**
- [x] **Android SDK Platform-Tools** (`adb`)
- [x] **NDK (Side by side)** (`{ndk}`)

By default on Linux, the SDK is installed in `~/Android/Sdk`.

---

## 5. Environment Variables

Set these variables so that command-line tools (`adb`, `cargo-ndk`, `gradlew`) can find the SDK.

### Bash (`~/.bashrc`) or Zsh (`~/.zshrc`)

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/{ndk}"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$HOME/.cargo/bin:$PATH"
```

### Fish Shell (`~/.config/fish/config.fish`)

```fish
set -gx ANDROID_HOME "$HOME/Android/Sdk"
set -gx ANDROID_SDK_ROOT "$ANDROID_HOME"
set -gx ANDROID_NDK_HOME "$ANDROID_HOME/ndk/{ndk}"
fish_add_path "$ANDROID_HOME/cmdline-tools/latest/bin" "$ANDROID_HOME/platform-tools" "$HOME/.cargo/bin"
```

After editing, reload your shell:
```bash
source ~/.bashrc # or source ~/.zshrc
```

---

## 6. Rust Toolchain for Android (`cargo-ndk` and Targets)

Since {name} has a shared Rust core, the compiler needs the Android targets installed:

```bash
# Add the Rust cross-compilation targets
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android

# Install the cargo-ndk utility
cargo install cargo-ndk
```

### Automated Script

The repository ships a script that checks and configures these dependencies automatically:

```bash
./scripts/setup-android-env.sh
```

---

## 7. Verifying the Environment

To check that everything is configured correctly:

1. Run the jirau diagnostics:
   ```bash
   jirau doctor
   ```
2. Test `adb`:
   ```bash
   adb devices
   ```
3. Test `cargo-ndk`:
   ```bash
   cargo ndk --version
   ```
4. Test the Rust compiler targets:
   ```bash
   rustup target list --installed
   ```
5. Run the first Gradle build:
   ```bash
   cd android && ./gradlew assembleDebug
   ```
"#,
        name = ctx.name,
        java = versions::JAVA_LTS,
        rust = versions::RUST,
        sdk = versions::ANDROID_COMPILE_SDK,
        build_tools = versions::ANDROID_BUILD_TOOLS,
        ndk = versions::ANDROID_NDK,
    )
}

pub fn setup_android_env_sh(ctx: &TemplateContext) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

# Garante que ~/.cargo/bin e ~/.local/bin estejam no PATH se existirem
if [ -d "$HOME/.cargo/bin" ] && [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi
if [ -d "$HOME/.local/bin" ] && [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    export PATH="$HOME/.local/bin:$PATH"
fi

echo "=========================================================="
echo "  🪵 {name} - Verificação & Setup do Ambiente Android"
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
if [ -n "${{ANDROID_HOME:-}}" ] || [ -n "${{ANDROID_SDK_ROOT:-}}" ]; then
    SDK_DIR="${{ANDROID_HOME:-$ANDROID_SDK_ROOT}}"
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
"#,
        name = ctx.name
    )
}

pub fn root_ci_workflow(ctx: &TemplateContext) -> String {
    let has_android = ctx.platforms.contains(&"android".to_string());
    let has_ios = ctx.platforms.contains(&"ios".to_string());

    let mut android_job = String::new();
    if has_android {
        android_job = format!(
            r#"
  android:
    name: Android (Rust core + APK + unit tests)
    runs-on: ubuntu-latest
    steps:
      - uses: {checkout}

      - name: Set up JDK {java}
        uses: {setup_java}
        with:
          distribution: temurin
          java-version: '{java}'

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-linux-android

      - uses: {rust_cache}

      - name: Set up Gradle
        uses: {setup_gradle}

      # O Gradle chama scripts/build-android-ffi.sh (cargo-ndk + bindings Kotlin).
      - name: Build APK & run unit tests
        working-directory: android
        run: ./gradlew assembleDebug testDebugUnitTest -Pjirau.rustAbis=x86_64
"#,
            checkout = versions::ACTION_CHECKOUT,
            setup_java = versions::ACTION_SETUP_JAVA,
            rust_cache = versions::ACTION_RUST_CACHE,
            setup_gradle = versions::ACTION_GRADLE,
            java = versions::JAVA_LTS,
        );
    }

    let mut ios_job = String::new();
    if has_ios {
        ios_job = format!(
            r#"
  ios:
    name: iOS (Rust core XCFramework + app build)
    runs-on: macos-latest
    steps:
      - uses: {checkout}

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-ios, aarch64-apple-ios-sim

      - uses: {rust_cache}

      - name: Install XcodeGen
        run: brew install xcodegen

      - name: Build Rust core XCFramework & Swift bindings
        run: ./scripts/build-ios-ffi.sh debug

      - name: Generate Xcode project & build
        run: |
          cd ios
          xcodegen generate
          xcodebuild -project {pascal_name}.xcodeproj -scheme {pascal_name} -configuration Debug -destination 'generic/platform=iOS Simulator' build CODE_SIGNING_ALLOWED=NO
"#,
            checkout = versions::ACTION_CHECKOUT,
            rust_cache = versions::ACTION_RUST_CACHE,
            pascal_name = ctx.pascal_name
        );
    }

    format!(
        r#"name: CI

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]

jobs:
  core:
    name: Rust core (fmt, clippy, tests)
    runs-on: ubuntu-latest
    steps:
      - uses: {checkout}

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - uses: {rust_cache}

      - name: Format check
        run: cargo fmt --all --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Tests
        run: cargo test --workspace
{android_job}{ios_job}"#,
        checkout = versions::ACTION_CHECKOUT,
        rust_cache = versions::ACTION_RUST_CACHE,
        android_job = android_job,
        ios_job = ios_job
    )
}
