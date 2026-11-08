<div align="center">

# 🪵 jirau

**CLI de scaffold nativo multiplataforma: Rust + Android (Compose) + iOS (SwiftUI) + Desktop (Rune UI)**

*A agilidade do `flutter create` + `flutter doctor` + `flutter run` com a pureza da UI 100% nativa e apps desktop de alta performance em Rust.*

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Android](https://img.shields.io/badge/Android-Jetpack%20Compose-green.svg?logo=android)](https://developer.android.com/compose)
[![iOS](https://img.shields.io/badge/iOS-SwiftUI-black.svg?logo=apple)](https://developer.apple.com/xcode/swiftui/)
[![Desktop](https://img.shields.io/badge/Desktop-Rune%20UI-blueviolet.svg?logo=rust)](https://github.com/anomalyco/rune)
[![UniFFI](https://img.shields.io/badge/FFI-Mozilla%20UniFFI-purple.svg)](https://mozilla.github.io/uniffi-rs/)
[![Release](https://img.shields.io/github/v/release/lucaswilliameufrasio/jirau?color=blue&logo=github)](https://github.com/lucaswilliameufrasio/jirau/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/lucaswilliameufrasio/jirau/ci.yml?branch=main&label=CI&logo=githubactions)](https://github.com/lucaswilliameufrasio/jirau/actions/workflows/ci.yml)
[![Status](https://img.shields.io/badge/status-v0.3.0%20ativo-success.svg)](#roadmap)

</div>

---

## 💡 O que é o Jirau?

Nas casas ribeirinhas da Amazônia, o **jirau** é um estrado de madeira suspenso, feito para apoiar, organizar e manter coisas a salvo. É um andaime firme e prático.

Essa metáfora define a ferramenta: o **jirau** monta a estrutura robusta sobre a qual seu aplicativo móvel se apoia e **sai do caminho**. O projeto gerado não fica amarrado à CLI, não inclui runtimes proprietários e pode ser compilado normalmente com `cargo`, Gradle e Xcode.

---

## ⚡ Por que jirau?

Abordagens tradicionais de compartilhamento de código ou te forçam a abrir mão da experiência nativa (Flutter/React Native) ou impõem arquiteturas opinativas e pesadas (Crux, Mobiler):

- 🦀 **O núcleo em Rust tem apenas lógica:** Nada de gerenciar ciclo de vida da UI ou orquestrar loops de efeitos colaterais complexos dentro do Rust. Testes unitários do core rodam no host com `cargo test` em milissegundos.
- 📱 **UI Mobile 100% nativa e idiomática:**
  - **Android:** Kotlin puro, Jetpack Compose, Navigation Compose, `ViewModel` e `StateFlow`.
  - **iOS:** Swift puro, SwiftUI, `NavigationStack` e `@Observable` (suporte exclusivo a Apple Silicon).
- 🖥️ **Desktop nativo em Rust com Rune:**
  - Aplicações desktop aceleradas por GPU via [Rune UI](https://github.com/anomalyco/rune).
  - Reutiliza `<app>-core` diretamente em Rust, **sem camada de FFI** e sem custo de conversão.
  - Disponível no Linux, Windows e macOS (Apple Silicon). *Nota: macOS Intel (`x86_64`) não é suportado.*
- 🔗 **Ponte via Mozilla UniFFI puro:** Sem JNI artesanal em C/C++, sem camadas frágeis intermediárias. Tipos exportados com segurança entre Rust, Kotlin e Swift para mobile.
- 🚫 **Zero Vendor Lock-in:** O `jirau` é um acelerador de produtividade, não um framework de execução. Seu time continua usando o Android Studio, o Xcode e o Cargo padrão.

---

## 📂 Estrutura do Projeto Gerado

Ao executar `jirau new meu-app`, um workspace Cargo modular é criado com os apps nativos como pastas irmãs:

```text
meu-app/
├── Cargo.toml               # Workspace Cargo unificado
├── crates/
│   ├── meu-app-core/        # Lógica de negócio pura (sem FFI, SQLite opcional com rusqlite)
│   ├── meu-app-ffi/         # Definições e proc-macros do Mozilla UniFFI (mobile)
│   └── meu-app-desktop/     # (Opcional) App Desktop em Rust puro com Rune UI
├── android/                 # (Opcional) Projeto Android nativo Jetpack Compose
│   ├── settings.gradle.kts
│   ├── gradle.properties    # Otimizado com configuration-cache e paralelo
│   ├── gradle/libs.versions.toml
│   ├── module.yaml          # (Opcional) Configuração declarativa JetBrains Amper
│   ├── project.yaml         # (Opcional) Catálogo de módulos Amper
│   └── app/                 # UI em Jetpack Compose, ViewModels e StateFlow
├── ios/                     # (Opcional) Projeto iOS nativo SwiftUI
│   ├── project.yml          # XcodeGen (declarativo, livre de conflitos no .pbxproj)
│   └── App/                 # UI em SwiftUI com NavigationStack e @Observable
├── jirau.toml               # Metadados do app (nome, bundle id, plataformas, persistência e build)
└── TESTING_STANDARD.md      # Padrões e diretrizes de testes do ecossistema
```

### Princípios da arquitetura:
1. **Separação estrita entre `core` e consumidores:** O `core` contém apenas as regras de negócio em Rust puro. O app desktop consome o `core` diretamente; as plataformas móveis consomem via `ffi`.
2. **Desktop em Rust puro:** A crate `crates/<slug>-desktop` usa o framework [Rune](https://github.com/anomalyco/rune), aproveitando a GPU com renderização moderna e reatividade de alta performance.
3. **Persistência local transparente:** Suporte opcional a SQLite embutido via `rusqlite` (`--sqlite`), com `StorageEngine` no core Rust e bindings nativos imediatos para Kotlin e Swift.
4. **Build Android flexível:** Suporte tanto ao tradicional **Gradle (Kotlin DSL)** com caches agressivos quanto ao moderno **JetBrains Amper** declarativo (`--amper`).
5. **Artefatos gerados fora do versionamento:** Bindings de Kotlin (`build/generated/uniffi`) e Swift (`ios/Generated/`) são gerados automaticamente.
6. **Bibliotecas Nativas Mobile:** Gera `.so` distribuídos por ABI em `jniLibs/` no Android e um `XCFramework` unificado (Device + Simulador) no iOS.
7. **XcodeGen:** O projeto iOS é configurado via `project.yml`, permitindo inspecionar e gerar a estrutura até mesmo a partir do Linux.
8. **Evolução contínua sem quebras:** O comando `jirau upgrade` sincroniza as novidades do template entre versões usando algoritmo de 3-way merge, preservando intacto todo o código customizado pelo desenvolvedor.

---

## 🛠️ Comandos da CLI

Fluxo direto inspirado no ecossistema Flutter:

| Comando | O que faz | Equivalente Flutter |
| :--- | :--- | :--- |
| `jirau new <nome>` | Cria um projeto interativo ou via flags (`--android`, `--ios`, `--desktop`, `--sqlite`, `--amper`). | `flutter create` |
| `jirau add <plataforma>` | Adiciona uma plataforma a um projeto existente (`jirau add desktop`, `android`, `ios`). | `flutter create . --platforms` |
| `jirau doctor` | Valida rustup, targets móveis, NDK, JDK, Xcode, Amper e dependências Desktop (Rune/GPU). | `flutter doctor` |
| `jirau dev [android\|ios\|desktop]` | Hot loop: compila e executa o app com recarregamento/reinício rápido ao alterar `crates/`. | `flutter run` |
| `jirau bindings` | Regera os wrappers Kotlin e Swift a partir das anotações em `-ffi`. | `build_runner` |
| `jirau build [android\|ios\|desktop] --release` | Compila os binários finais (APK/AAB, XCFramework ou binário de desktop otimizado). | `flutter build` |
| `jirau upgrade [--force]` | Sincroniza melhorias do template via 3-way merge preservando modificações manuais. | `flutter upgrade` |

### Otimizações do `jirau dev`
- **Detecção automática de ABI:** Inspeciona o emulador ou aparelho conectado (`adb shell getprop ro.product.cpu.abi`) e compila apenas o target necessário (`arm64-v8a` no celular físico ou `x86_64` no emulador).
- **Cache agressivo:** Integração com `sccache` e tasks inteligentes no Gradle com inputs (`crates/**`, `Cargo.lock`) e outputs (`jniLibs/`) declarados.
- **Ciclo curto:** Menos de 10 segundos entre salvar um arquivo `.rs` e ver a alteração rodando no app de teste.

---

## 🚀 Instalação

Você pode instalar os binários pré-compilados diretamente via script oficial:

### Linux e macOS
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/lucaswilliameufrasio/jirau/releases/download/v0.1.0/jirau-installer.sh | sh
```

### Windows (PowerShell)
```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/lucaswilliameufrasio/jirau/releases/download/v0.1.0/jirau-installer.ps1 | iex"
```

### Via Cargo
```bash
cargo install --git https://github.com/lucaswilliameufrasio/jirau.git
```

---

## 📋 Pré-requisitos & Ferramentas

O `jirau doctor` diagnostica e ajuda a instalar os itens necessários:

- **Rust:** `rustup` com toolchain stable.
- **Targets Rust Mobile:**
  - Android: `aarch64-linux-android`, `x86_64-linux-android`
  - iOS: `aarch64-apple-ios`, `aarch64-apple-ios-sim`
- **Android:** Android SDK, NDK recente, JDK 17+ e `cargo-ndk`.
- **iOS:** macOS com Apple Silicon (M1/M2/M3/M4) e Xcode instalado com `xcodegen` (`brew install xcodegen`). *(Intel macOS descontinuado)*.
- **Desktop (Rune UI):**
  - **Linux:** Bibliotecas de desenvolvimento de janela/GPU (`pkg-config`, `libxkbcommon-dev`, `libfontconfig1-dev` no Debian/Ubuntu).
  - **macOS:** Apple Silicon (`aarch64-apple-darwin`) com Metal nativo.
  - **Windows:** Suporte nativo a DirectX 12 / Vulkan via MSVC.

---

## 🌟 Projetos de Exemplo

O repositório inclui uma pasta [`examples/`](examples/) com projetos gerados diretamente via CLI para demonstrar cada combinação de ambiente:

- **[`all-platforms-showcase`](examples/all-platforms-showcase/)**: App com **todas as plataformas ativas** (Android Jetpack Compose + iOS SwiftUI + Desktop Rune UI), persistência SQLite embutida (`rusqlite`), JetBrains Amper declarativo, kit de design Material 3 e testes de ciclo de vida FFI.
- **[`mobile-gradle-starter`](examples/mobile-gradle-starter/)**: Arquitetura mobile pura (Android + iOS) usando Gradle (Kotlin DSL) padrão e estado em memória reativo.
- **[`desktop-first-sqlite`](examples/desktop-first-sqlite/)**: Aplicativo Desktop nativo em Rust consumindo diretamente o `StorageEngine` SQLite sem ponte de FFI.

Consulte o [`examples/README.md`](examples/README.md) para instruções detalhadas de como executá-los.

---

## 🗺️ Roadmap de Desenvolvimento

- [x] **Fase 0: Concepção & Arquitetura** ([`docs/development-plan.md`](docs/development-plan.md))
- [x] **Fase 1: Extração e Consolidação de Templates** (UniFFI, Compose, SwiftUI, XcodeGen `project.yml`, caches do Gradle e Cargo)
- [x] **Fase 2: CLI Core Guiada (`new` e `doctor`)** (assistente passo a passo com `inquire`, diagnóstico de ambiente com sugestões exatas de correção e suporte a `--json`)
- [x] **Fase 3: Hot Loop Android (`dev` & `bindings`)** (detecção automática de ABI por `adb`, compilação incremental de ABI única via `cargo-ndk`, file watcher com `notify` e bindings Kotlin)
- [x] **Fase 4: Suporte iOS (`dev`, `bindings` & `build`)** (integração com XcodeGen declarativo, XCFramework e simuladores)
- [x] **Fase 5: Dogfooding & Suíte de Testes** (testes unitários e teste de integração ponta a ponta gerando apps temporários e validando `cargo test`)
- [x] **Fase 6: Automação de Release & Distribuição** (pipeline `cargo-dist`, changelog automático via `git-cliff`, binários para Linux, macOS e Windows na v0.1.0)
- [x] **Fase 7: Persistência & Amper (v0.2.0)** (SQLite embutido no core via `rusqlite`, suporte declarativo ao JetBrains Amper e comando `jirau upgrade` com 3-way merge)
- [x] **Fase 8: Temas, Plugins & Testes FFI End-to-End (v0.3.0)** (tokens de design para Compose/SwiftUI, suíte de testes de lifecycle nativo multithread no host e vitrine de exemplos em `examples/`)

---

## 📄 Licença

Distribuído sob a licença MIT ou Apache 2.0. Consulte `LICENSE` para mais detalhes.
