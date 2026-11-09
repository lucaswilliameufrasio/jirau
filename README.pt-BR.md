<div align="center">

[English](README.md) | Português (Brasil)

# 🪵 jirau

**CLI de scaffold nativo multiplataforma: Rust + Android (Compose) + iOS (SwiftUI) + Desktop (Rune UI)**

*A agilidade do `flutter create` + `flutter doctor` + `flutter run` com a pureza da UI 100% nativa e apps desktop de alta performance em Rust.*

[![Rust](https://img.shields.io/badge/rust-1.99.1%20(edition%202024)-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Android](https://img.shields.io/badge/Android-Jetpack%20Compose-green.svg?logo=android)](https://developer.android.com/compose)
[![iOS](https://img.shields.io/badge/iOS-SwiftUI-black.svg?logo=apple)](https://developer.apple.com/xcode/swiftui/)
[![Desktop](https://img.shields.io/badge/Desktop-Rune%20UI-blueviolet.svg?logo=rust)](https://github.com/anomalyco/rune)
[![UniFFI](https://img.shields.io/badge/FFI-Mozilla%20UniFFI%200.32-purple.svg)](https://mozilla.github.io/uniffi-rs/)
[![Release](https://img.shields.io/github/v/release/lucaswilliameufrasio/jirau?color=blue&logo=github)](https://github.com/lucaswilliameufrasio/jirau/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/lucaswilliameufrasio/jirau/ci.yml?branch=main&label=CI&logo=githubactions)](https://github.com/lucaswilliameufrasio/jirau/actions/workflows/ci.yml)
[![Template](https://img.shields.io/badge/template-v0.4.0-success.svg)](#roadmap)

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
  - **Android:** Kotlin puro, Jetpack Compose, `ViewModel` e `StateFlow`.
  - **iOS:** Swift puro (modo de linguagem Swift 6), SwiftUI, `NavigationStack` e `@Observable` (iOS 17+, apenas Apple Silicon).
- 🖥️ **Desktop nativo em Rust com Rune:**
  - Aplicações desktop aceleradas por GPU via [Rune UI](https://github.com/anomalyco/rune).
  - Reutiliza `<app>-core` diretamente em Rust, **sem camada de FFI** e sem custo de conversão.
  - Disponível no Linux, Windows e macOS (Apple Silicon). *Nota: macOS Intel (`x86_64`) não é suportado.*
- 🔗 **Ponte via Mozilla UniFFI puro:** Sem JNI artesanal em C/C++, sem camadas frágeis intermediárias. O núcleo Rust é linkado de verdade nas duas plataformas pelo modo *library* do UniFFI, com tipos exportados com segurança entre Rust, Kotlin e Swift.
- 🚫 **Zero Vendor Lock-in:** O `jirau` é um acelerador de produtividade, não um framework de execução. Seu time continua usando o Android Studio, o Xcode e o Cargo padrão.

---

## 📂 Estrutura do Projeto Gerado

Ao executar `jirau new meu-app`, um workspace Cargo modular é criado com os apps nativos como pastas irmãs:

```text
meu-app/
├── Cargo.toml               # Workspace Cargo unificado
├── .mise.toml               # Toolchains fixadas (JDK 25 LTS, Rust 1.99.1)
├── Makefile                 # Atalhos: setup, doctor, test, android-ffi, ios-ffi, ...
├── crates/
│   ├── meu-app-core/        # Lógica de negócio pura (sem FFI, SQLite opcional com rusqlite)
│   ├── meu-app-ffi/         # Definições e proc-macros do Mozilla UniFFI (mobile)
│   └── meu-app-desktop/     # (Opcional) App Desktop em Rust puro com Rune UI
├── android/                 # (Opcional) Projeto Android nativo Jetpack Compose (Gradle)
│   ├── gradlew              # Gradle Wrapper incluso (distribuição fixada por SHA-256)
│   ├── settings.gradle.kts
│   ├── gradle.properties    # Otimizado com configuration cache e build paralelo
│   ├── gradle/libs.versions.toml
│   └── app/                 # UI em Jetpack Compose, ViewModels e StateFlow
├── ios/                     # (Opcional) Projeto iOS nativo SwiftUI
│   ├── project.yml          # XcodeGen (declarativo, livre de conflitos no .pbxproj)
│   └── App/                 # UI em SwiftUI com NavigationStack e @Observable
├── scripts/
│   ├── build-android-ffi.sh # cargo-ndk → jniLibs/ + bindings Kotlin
│   ├── build-ios-ffi.sh     # XCFramework + bindings Swift
│   └── setup-android-env.sh # Verifica/instala os pré-requisitos do Android
├── docs/setup-android.md    # Guia do ambiente Android (Linux, headless ou Android Studio)
├── .github/workflows/ci.yml # (Opcional, --ci) Jobs de core, Android e iOS
├── .jirau/template-base/    # Cópia exata do template gerado (base do `jirau upgrade`)
├── jirau.toml               # Metadados do app (nome, bundle id, plataformas, persistência, versão do template)
└── TESTING_STANDARD.md      # Padrões e diretrizes de testes
```

### Princípios da arquitetura
1. **Separação estrita entre `core` e consumidores:** O `core` contém apenas as regras de negócio em Rust puro. O app desktop consome o `core` diretamente; as plataformas móveis consomem via `ffi`.
2. **Desktop em Rust puro:** A crate `crates/<slug>-desktop` usa o framework [Rune](https://github.com/anomalyco/rune), aproveitando a GPU com renderização moderna e reatividade de alta performance.
3. **Persistência local transparente:** Suporte opcional a SQLite embutido via `rusqlite` (`--sqlite`), com `StorageEngine` no core Rust e bindings nativos para Kotlin e Swift.
4. **Gradle é o sistema de build do Android:** Kotlin DSL com caches agressivos e wrapper incluso. O suporte ao JetBrains Amper foi removido (o modo plugin do Gradle foi descontinuado e o Amper standalone ainda não tem versão estável 1.0); `--amper` agora falha com uma explicação.
5. **Um só pipeline de build:** `scripts/build-ios-ffi.sh` e `scripts/build-android-ffi.sh` são os mesmos scripts usados pelo jirau, pelo `make`, pela CI, pelo Android Studio e pelo Xcode. O Gradle roda o script do Android automaticamente no `preBuild` (escolha as ABIs com `-Pjirau.rustAbis=x86_64`).
6. **Artefatos gerados fora do versionamento:** Bindings Kotlin (`android/app/build/generated/uniffi`), bindings Swift (`ios/Generated/`), `jniLibs/` e o XCFramework são regerados automaticamente.
7. **Bibliotecas nativas mobile:** `.so` por ABI em `jniLibs/` no Android e um `XCFramework` unificado (device + simulador) no iOS.
8. **XcodeGen:** O projeto iOS é descrito em `project.yml`, permitindo inspecionar e gerar a estrutura até mesmo a partir do Linux. Builds para o simulador usam assinatura ad-hoc (`CODE_SIGN_IDENTITY[sdk=iphonesimulator*] = "-"`), porque o Keychain exige entitlements.
9. **Evolução contínua sem quebras:** O `jirau upgrade` faz um 3-way merge de verdade usando `.jirau/template-base/` como base, preservando o código customizado. Projetos sem base recebem arquivos `<arquivo>.jirau-new` ao lado para revisão manual.

---

## 🛠️ Comandos da CLI

Fluxo direto inspirado no ecossistema Flutter:

| Comando | O que faz | Equivalente Flutter |
| :--- | :--- | :--- |
| `jirau new <nome>` | Cria um projeto interativo ou via flags (`--android`, `--ios`, `--desktop`, `--sqlite`, `--template starter\|vault`, `--theme`, `--plugin`, `--ci`, `--non-interactive`). | `flutter create` |
| `jirau add <plataforma>` | Adiciona uma plataforma a um projeto existente (`jirau add desktop`, `android`, `ios`). | `flutter create . --platforms` |
| `jirau doctor` | Valida rustup, targets móveis, NDK, JDK, Android SDK, Xcode, XcodeGen e dependências do desktop (`--json` para CI, `--profile` para o build do Gradle). | `flutter doctor` |
| `jirau run [android\|ios\|desktop]` | Compila, instala e abre o app no dispositivo/simulador. Escolhe sozinho o único dispositivo conectado/ligado ou mostra um menu de seleção; `-d <id>` pula a seleção. O modo watch vem ligado por padrão (`--no-watch` desliga). | `flutter run` |
| `jirau dev [android\|ios\|desktop]` | Loop rápido de desenvolvimento: recompila e reinstala quando `crates/` muda. | `flutter run` (hot restart) |
| `jirau test [core\|ffi\|android\|ios\|desktop\|all]` | Suíte unificada de testes: Rust Core, lifecycle FFI, testes instrumentados Android no dispositivo conectado, XCTest no simulador iOS e desktop. | `flutter test` |
| `jirau bindings [android\|ios]` | Regera os wrappers Kotlin e Swift a partir das anotações em `-ffi`. | `build_runner` |
| `jirau build [android\|ios\|desktop]` | Compila os artefatos finais (APK multi-ABI, XCFramework ou binário desktop otimizado). | `flutter build` |
| `jirau upgrade [--force] [--dry-run]` | Aplica melhorias do template via 3-way merge, preservando modificações manuais. | `flutter upgrade` |

**Idioma:** todos os comandos aceitam `--lang en|pt-BR`. Também é possível definir `JIRAU_LANG`; por padrão o jirau segue o idioma do sistema. Mensagens, a ajuda (`--help`) e os erros de argumentos são todos traduzidos.

### Otimizações do loop de desenvolvimento
- **Detecção automática de ABI:** Inspeciona o emulador ou aparelho conectado (`adb shell getprop ro.product.cpu.abi`) e compila apenas o target necessário (`arm64-v8a` no celular físico ou `x86_64` no emulador).
- **Um `target/` compartilhado:** Um só workspace Cargo para Android, iOS e testes no host, com dependências otimizadas no perfil `dev`.

---

## 🧩 Templates de App

| Template | O que você recebe |
| :--- | :--- |
| `starter` (padrão) | Um contador e uma saudação calculados pelo núcleo Rust e exibidos pelas UIs nativas. |
| `vault` | Um gerenciador de senhas: armazenamento SQLite com segredos cifrados com AES-256-GCM-SIV via `aws-lc-rs`. A chave de 32 bytes é gerada pelo núcleo Rust e guardada no Keychain do iOS ou protegida (wrapped) pelo Android Keystore. Bancos legados em texto puro são migrados automaticamente. Títulos, usuários e tags continuam como metadados em texto puro, pesquisáveis. |

---

## 📌 Versões Fixadas das Tecnologias

Todas as versões usadas pelos projetos gerados ficam numa única fonte, [`src/templates/versions.rs`](src/templates/versions.rs). O [`scripts/check-template-versions.sh`](scripts/check-template-versions.sh) compara essas versões com os registros oficiais e faz a CI falhar quando alguma está desatualizada.

| Área | Versão |
| :--- | :--- |
| Rust | 1.99.1 (edition 2024) |
| JDK | 25 (LTS) |
| Gradle | 9.9.0 (wrapper incluso, SHA-256 da distribuição verificado) |
| Android Gradle Plugin | 9.5.1 |
| Kotlin | 2.4.21 |
| Android SDK | compileSdk/targetSdk 37, minSdk 26 |
| Compose BOM | 2026.09.00 |
| Android NDK | 30.0.16248370 |
| UniFFI | 0.32 |
| rusqlite | 0.41 |
| aws-lc-rs | 1.19 |
| Swift | modo de linguagem Swift 6 |
| iOS | deployment target 17+ |
| Xcode | 27 |

A versão atual do template é a **0.4.0**.

---

## 🚀 Instalação

Você pode instalar os binários pré-compilados com os scripts oficiais:

### Linux e macOS
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/lucaswilliameufrasio/jirau/releases/latest/download/jirau-installer.sh | sh
```

### Windows (PowerShell)
```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/lucaswilliameufrasio/jirau/releases/latest/download/jirau-installer.ps1 | iex"
```

### Via Cargo
```bash
cargo install --git https://github.com/lucaswilliameufrasio/jirau.git
```

---

## 📋 Pré-requisitos & Ferramentas

O `jirau doctor` diagnostica e ajuda a instalar os itens necessários:

- **Rust:** `rustup` (o `.mise.toml` gerado fixa o Rust 1.99.1).
- **Targets Rust Mobile:**
  - Android: `aarch64-linux-android`, `x86_64-linux-android`
  - iOS: `aarch64-apple-ios`, `aarch64-apple-ios-sim`
- **Android:** Android SDK (plataforma 37), NDK 30.0.16248370, JDK 25 e `cargo-ndk`. Veja [`docs/setup-android.pt-BR.md`](docs/setup-android.pt-BR.md).
- **iOS:** macOS com Apple Silicon, Xcode 27 e `xcodegen` (`brew install xcodegen`). *(macOS Intel não é suportado.)*
- **Desktop (Rune UI):**
  - **Linux:** Bibliotecas de desenvolvimento de janela/GPU (`pkg-config`, `libxkbcommon-dev`, `libfontconfig1-dev` no Debian/Ubuntu).
  - **macOS:** Apple Silicon (`aarch64-apple-darwin`) com Metal nativo.
  - **Windows:** Suporte nativo a DirectX 12 / Vulkan via MSVC.

---

## 🌟 Projetos de Exemplo

A pasta [`examples/`](examples/) contém projetos regerados a partir dos templates para demonstrar cada combinação:

- **[`secure-vault`](examples/secure-vault/)**: App real e funcional de **gerenciador de senhas e cofre local** (o template `vault`): SQLite cifrado, busca em tempo real, gerador criptográfico de senhas e medidor de entropia no core Rust, além das UIs em Jetpack Compose, SwiftUI e Rune desktop. O template `vault` é sincronizado a partir deste exemplo com `scripts/sync-vault-template.py`.
- **[`all-platforms-showcase`](examples/all-platforms-showcase/)**: App com **todas as plataformas ativas** (Android Jetpack Compose + iOS SwiftUI + Desktop Rune UI), SQLite embutido (`rusqlite`), tema Material 3 estendido e testes de ciclo de vida FFI.
- **[`mobile-gradle-starter`](examples/mobile-gradle-starter/)**: Arquitetura só mobile (Android + iOS) com Gradle (Kotlin DSL) padrão e estado em memória reativo.
- **[`desktop-first-sqlite`](examples/desktop-first-sqlite/)**: App Desktop nativo em Rust usando diretamente o `StorageEngine` SQLite, sem ponte de FFI.

Validado de ponta a ponta no simulador iOS (iPhone 18 Pro, Xcode 27.2) e no emulador Android (API 36, x86_64): `run`, testes, persistência e criptografia.

Consulte o [`examples/README.md`](examples/README.md) para instruções detalhadas de como executá-los.

---

## 🗺️ Roadmap

- [x] **Fase 0: Concepção & Arquitetura** ([`docs/development-plan.pt-BR.md`](docs/development-plan.pt-BR.md))
- [x] **Fase 1: Extração e Consolidação de Templates** (UniFFI, Compose, SwiftUI, XcodeGen `project.yml`, caches do Gradle e Cargo)
- [x] **Fase 2: CLI Core Guiada (`new` e `doctor`)** (assistente passo a passo com `inquire`, diagnóstico de ambiente com sugestões exatas de correção e suporte a `--json`)
- [x] **Fase 3: Hot Loop Android (`dev` & `bindings`)** (detecção automática de ABI por `adb`, compilação incremental de ABI única via `cargo-ndk`, file watcher com `notify` e bindings Kotlin)
- [x] **Fase 4: Suporte iOS (`dev`, `bindings` & `build`)** (integração com XcodeGen declarativo, XCFramework e simuladores)
- [x] **Fase 5: Dogfooding & Suíte de Testes** (testes unitários e teste de integração ponta a ponta gerando apps temporários e validando `cargo test`)
- [x] **Fase 6: Automação de Release & Distribuição** (pipeline `cargo-dist`, changelog automático via `git-cliff`, binários para Linux, macOS e Windows na v0.1.0)
- [x] **Fase 7: Persistência & Upgrades (v0.2.0)** (SQLite embutido no core via `rusqlite` e comando `jirau upgrade`; o suporte ao JetBrains Amper adicionado nesta fase foi removido depois)
- [x] **Fase 8: Temas, Plugins & Testes FFI End-to-End (v0.3.0)** (tokens de design para Compose/SwiftUI, suíte de testes de lifecycle nativo multithread no host e vitrine de exemplos em `examples/`)
- [x] **Fase 9: Link Nativo Real & Dispositivos (template v0.4.0)** (núcleo Rust linkado via modo *library* do UniFFI nas duas plataformas, `jirau run` e `jirau test` no dispositivo, template `vault` com criptografia, versões fixadas e conferidas nos registros oficiais, `upgrade` com 3-way merge real, CLI em inglês e português)

---

## 📄 Licença

Distribuído sob a licença MIT ou Apache 2.0 (`MIT OR Apache-2.0`).
