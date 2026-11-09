[English](CHANGELOG.md) | Português (Brasil)

# Changelog

Todas as mudanças relevantes deste projeto são documentadas neste arquivo.

## [Não lançado]

Versão do template **0.4.0**.

### Funcionalidades
- Núcleo Rust linkado de verdade nas duas plataformas pelo modo *library* do UniFFI: `scripts/build-ios-ffi.sh` (XCFramework + bindings Swift, projeto Xcode gerado pelo XcodeGen a partir de `ios/project.yml`) e `scripts/build-android-ffi.sh` (cargo-ndk → `jniLibs/` + bindings Kotlin). O Gradle roda o script do Android automaticamente no `preBuild` (ABIs via `-Pjirau.rustAbis=x86_64`). Os mesmos scripts são usados pelo jirau, pelo `make`, pela CI, pelo Android Studio e pelo Xcode.
- `jirau run`: compila, instala e abre o app no dispositivo/simulador, com seleção automática de dispositivo, modo watch por padrão (`--no-watch`) e `-d <id>`.
- `jirau test`: executor unificado para `core`, `ffi`, `android` (testes instrumentados no dispositivo conectado), `ios` (XCTest no simulador), `desktop` e `all`.
- `jirau upgrade`: 3-way merge real usando `.jirau/template-base/` como base; projetos sem base recebem arquivos `<arquivo>.jirau-new` para revisão. Aceita `--force` e `--dry-run`.
- Novo template `vault` (gerenciador de senhas): armazenamento SQLite com segredos cifrados com AES-256-GCM-SIV via `aws-lc-rs`; chave de 32 bytes gerada pelo núcleo Rust e guardada no Keychain do iOS ou protegida (wrapped) pelo Android Keystore; bancos legados em texto puro são migrados automaticamente; títulos, usuários e tags continuam como metadados em texto puro, pesquisáveis. O template `starter` (contador + saudação vindos do Rust) continua sendo o padrão.
- CLI bilíngue: opção global `--lang en|pt-BR`, variável de ambiente `JIRAU_LANG` e, por padrão, o idioma do sistema.
- Versões das tecnologias centralizadas em `src/templates/versions.rs` e conferidas nos registros oficiais por `scripts/check-template-versions.sh` (falha a CI se houver versão desatualizada): Rust 1.99.1 (edition 2024), JDK 25 (LTS), Gradle 9.9.0 (wrapper incluso com SHA-256 da distribuição), AGP 9.5.1, Kotlin 2.4.21, compileSdk/targetSdk 37, minSdk 26, Compose BOM 2026.09.00, NDK 30.0.16248370, UniFFI 0.32, rusqlite 0.41, aws-lc-rs 1.19, modo de linguagem Swift 6, deployment target iOS 17+, Xcode 27.
- Versões em inglês dos templates gerados `TESTING_STANDARD.md` e `docs/setup-android.md`.

### Mudanças
- Builds para o simulador iOS usam assinatura ad-hoc (`CODE_SIGN_IDENTITY[sdk=iphonesimulator*] = "-"`), porque o Keychain exige entitlements.
- Os exemplos em `examples/` (secure-vault, mobile-gradle-starter, all-platforms-showcase, desktop-first-sqlite) são regerados a partir dos templates; `scripts/sync-vault-template.py` sincroniza o template `vault` a partir de `examples/secure-vault`.

### Removido
- Suporte ao JetBrains Amper: o modo plugin do Gradle foi descontinuado e o Amper standalone não tem versão estável 1.0. `--amper` agora falha com uma explicação; o Gradle é o único sistema de build do Android.

### Validação
- Simulador iOS (iPhone 18 Pro, Xcode 27.2) e emulador Android (API 36, x86_64): `run`, testes, persistência e criptografia.

### Documentação
- A documentação agora é bilíngue: inglês (padrão) e português do Brasil (`*.pt-BR.md`), atualizada para refletir o estado atual.

## [0.3.0] - 2026-11-09

### Funcionalidades
- **Seleção inteligente de dispositivo em `jirau run [plataforma]`**: roda direto quando há um único dispositivo Android conectado ou simulador iOS ligado; abre um seletor interativo quando há vários; mostra orientações de configuração quando não há nenhum.
- **Executor de testes unificado `jirau test [alvo]`**: orquestra os testes do núcleo Rust, os testes de integração do ciclo de vida da FFI, os testes unitários JUnit 5 do Android, o XCTest do iOS e a compilação do desktop com Rune.
- **App de exemplo "Secure Vault" pronto para produção**: gerenciador de senhas completo com SQLite (esquema relacional e índices), busca e categorias, gerador criptográfico de senhas com avaliação de força, UI desktop com Rune na GPU, Jetpack Compose Material 3 e SwiftUI com `@Observable`.
- **Melhorias de ambiente e automação**: documentação e script automatizado de configuração headless do Android SDK, versões fixadas no `.mise.toml`, automação com Makefile na raiz e mais alternativas no `jirau doctor`.
- **Temas Material 3 Extended e plugins**: geradores de design tokens para Compose e SwiftUI, além da integração do plugin `logger`.
- **Geração de CI/CD**: a flag `--ci` adiciona um workflow do GitHub Actions pronto para uso (`.github/workflows/ci.yml`).

## [0.2.0] - 2026-11-08

### Funcionalidades
- **Suporte a desktop (Rune UI)**: apps desktop nativos renderizados na GPU via `anomalyco/rune`, consumindo o núcleo Rust diretamente, sem o custo da FFI.
- **Persistência com SQLite (`--sqlite`)**: banco relacional embarcado no núcleo via `rusqlite`, com consultas tipadas e bindings nativos.
- **Suporte ao JetBrains Amper (`--amper`)**: configuração declarativa em YAML para projetos Android.
- **Motor de atualização de template (`jirau upgrade`)**: algoritmo de merge 3-way para sincronizar projetos com novas versões do template preservando as customizações.
- **Projetos de referência**: `all-platforms-showcase`, `mobile-gradle-starter` e `desktop-first-sqlite` em `examples/`.

## [0.1.0] - 2026-11-07

### Funcionalidades
- Primeira versão da CLI jirau
- Fluxo guiado e interativo, com perguntas passo a passo, para `new`, `doctor`, `dev`, `bindings` e `build`
- Scaffold nativo completo para o núcleo Rust, a ponte UniFFI, Android (Jetpack Compose) e iOS (SwiftUI + XcodeGen)
- Motor de diagnóstico `doctor` com dicas de correção e suporte a `--json`
- Otimizações de build com detecção automática de ABI e file watcher

### Documentação
- README completo com arquitetura, comparação de fluxos e filosofia
- Plano de desenvolvimento e diretrizes de testes
