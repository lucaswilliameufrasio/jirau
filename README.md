<div align="center">

# 🪵 jirau

**CLI de scaffold nativo multiplataforma: Rust + Android (Compose) + iOS (SwiftUI)**

*A agilidade do `flutter create` + `flutter doctor` + `flutter run` com a pureza e a fidelidade da UI 100% nativa.*

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Android](https://img.shields.io/badge/Android-Jetpack%20Compose-green.svg?logo=android)](https://developer.android.com/compose)
[![iOS](https://img.shields.io/badge/iOS-SwiftUI-black.svg?logo=apple)](https://developer.apple.com/xcode/swiftui/)
[![UniFFI](https://img.shields.io/badge/FFI-Mozilla%20UniFFI-purple.svg)](https://mozilla.github.io/uniffi-rs/)
[![Status](https://img.shields.io/badge/status-em%20desenvolvimento-blue.svg)](#roadmap)

</div>

---

## 💡 O que é o Jirau?

Nas casas ribeirinhas da Amazônia, o **jirau** é um estrado de madeira suspenso, feito para apoiar, organizar e manter coisas a salvo. É um andaime firme e prático.

Essa metáfora define a ferramenta: o **jirau** monta a estrutura robusta sobre a qual seu aplicativo móvel se apoia e **sai do caminho**. O projeto gerado não fica amarrado à CLI, não inclui runtimes proprietários e pode ser compilado normalmente com `cargo`, Gradle e Xcode.

---

## ⚡ Por que jirau?

Abordagens tradicionais de compartilhamento de código ou te forçam a abrir mão da experiência nativa (Flutter/React Native) ou impõem arquiteturas opinativas e pesadas (Crux, Mobiler):

- 🦀 **O núcleo em Rust tem apenas lógica:** Nada de gerenciar ciclo de vida da UI ou orquestrar loops de efeitos colaterais complexos dentro do Rust. Testes unitários do core rodam no host com `cargo test` em milissegundos.
- 📱 **UI 100% nativa e idiomática:**
  - **Android:** Kotlin puro, Jetpack Compose, Navigation Compose, `ViewModel` e `StateFlow`.
  - **iOS:** Swift puro, SwiftUI, `NavigationStack` e `@Observable`.
- 🔗 **Ponte via Mozilla UniFFI puro:** Sem JNI artesanal em C/C++, sem camadas frágeis intermediárias. Tipos exportados com segurança entre Rust, Kotlin e Swift.
- 🚫 **Zero Vendor Lock-in:** O `jirau` é um acelerador de produtividade, não um framework de execução. Seu time continua usando o Android Studio e o Xcode com todas as ferramentas de preview e depuração ativas.

---

## 📂 Estrutura do Projeto Gerado

Ao executar `jirau new meu-app`, um workspace Cargo modular é criado com os apps nativos como pastas irmãs:

```text
meu-app/
├── Cargo.toml               # Workspace Cargo unificado
├── crates/
│   ├── meu-app-core/        # Lógica de negócio pura (sem FFI, testes ultra-rápidos)
│   └── meu-app-ffi/         # Definições e proc-macros do Mozilla UniFFI
├── android/                 # Projeto Android nativo
│   ├── settings.gradle.kts
│   ├── gradle.properties    # Otimizado com configuration-cache e paralelo
│   ├── gradle/libs.versions.toml
│   └── app/                 # UI em Jetpack Compose, ViewModels e StateFlow
├── ios/                     # Projeto iOS nativo
│   ├── project.yml          # XcodeGen (declarativo, livre de conflitos no .pbxproj)
│   └── App/                 # UI em SwiftUI com NavigationStack e @Observable
├── jirau.toml               # Metadados do app (nome, bundle id, ABIs de dev)
└── TESTING_STANDARD.md      # Padrões e diretrizes de testes do ecossistema
```

### Princípios da arquitetura:
1. **Separação estrita entre `core` e `ffi`:** A lógica (`core`) não sabe o que é UniFFI.
2. **Artefatos gerados fora do versionamento:** Bindings de Kotlin (`build/generated/uniffi`) e Swift (`ios/Generated/`) são gerados automaticamente.
3. **Bibliotecas Nativas:** Gera `.so` distribuídos por ABI em `jniLibs/` no Android e um `XCFramework` unificado (Device + Simulador) no iOS.
4. **XcodeGen:** O projeto iOS é configurado via `project.yml`, permitindo inspecionar e gerar a estrutura até mesmo a partir do Linux.

---

## 🛠️ Comandos da CLI

Fluxo direto inspirado no ecossistema Flutter:

| Comando | O que faz | Equivalente Flutter |
| :--- | :--- | :--- |
| `jirau new <nome>` | Cria um projeto a partir do template com pacote, bundle id e plataformas (`--android`, `--ios`). | `flutter create` |
| `jirau doctor` | Valida rustup, targets móveis, NDK, JDK, Android SDK, emuladores, Xcode, XcodeGen e sugere correções pontuais. | `flutter doctor` |
| `jirau dev [android\|ios]` | Observa `crates/`, compila apenas para a ABI conectada, regera bindings e atualiza o app ativo. | `flutter run` |
| `jirau bindings` | Regera os wrappers Kotlin e Swift a partir das anotações em `-ffi`. | `build_runner` |
| `jirau build [android\|ios] --release` | Compila os binários finais (AAB/APK com multi-ABI ou XCFramework assinado). | `flutter build` |

### Otimizações do `jirau dev`
- **Detecção automática de ABI:** Inspeciona o emulador ou aparelho conectado (`adb shell getprop ro.product.cpu.abi`) e compila apenas o target necessário (`arm64-v8a` no celular físico ou `x86_64` no emulador).
- **Cache agressivo:** Integração com `sccache` e tasks inteligentes no Gradle com inputs (`crates/**`, `Cargo.lock`) e outputs (`jniLibs/`) declarados.
- **Ciclo curto:** Menos de 10 segundos entre salvar um arquivo `.rs` e ver a alteração rodando no app de teste.

---

## 📋 Pré-requisitos & Ferramentas

O `jirau doctor` diagnostica e ajuda a instalar os itens necessários:

- **Rust:** `rustup` com toolchain stable.
- **Targets Rust:**
  - Android: `aarch64-linux-android`, `x86_64-linux-android`
  - iOS: `aarch64-apple-ios`, `aarch64-apple-ios-sim`
- **Android:** Android SDK, NDK recente, JDK 17+ e `cargo-ndk`.
- **iOS:** macOS com Xcode instalado e `xcodegen` (`brew install xcodegen`).

---

## 🗺️ Roadmap de Desenvolvimento

- [x] **Fase 0: Concepção & Arquitetura** ([`docs/development-plan.md`](docs/development-plan.md))
- [ ] **Fase 1: Extração do Template Base** (migração UniFFI, `project.yml`, ajustes de cache no Gradle)
- [ ] **Fase 2: CLI Core (`new` e `doctor`)** (geração de scaffolding e validação do ambiente)
- [ ] **Fase 3: Hot Loop Android (`dev` & `bindings`)** (file watcher, compilação de ABI única e push via adb)
- [ ] **Fase 4: Suporte iOS (`dev` & `build`)** (integração com XcodeGen, XCFramework e simctl)
- [ ] **Fase 5: Dogfooding & Validação Real** (validação de ponta a ponta na geração de apps satélites)
- [ ] **Fase 6: Release & Upgrades** (suporte a release multi-ABI e merge de template de 3 vias)

---

## 📄 Licença

Distribuído sob a licença MIT ou Apache 2.0. Consulte `LICENSE` para mais detalhes.
