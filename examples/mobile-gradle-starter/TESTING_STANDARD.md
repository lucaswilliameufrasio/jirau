# Diretrizes de Testes — mobile-gradle-starter

Este repositório adota a metodologia de testes rápidos em camada:

1. **Núcleo em Rust (`crates/mobile-gradle-starter-core`)**:
   - Testes unitários puros executados diretamente no host via `cargo test`.
   - Não dependem de emulador, simulador, Android NDK ou UniFFI.
   - Meta de execução: < 500 ms.

2. **Camada FFI (`crates/mobile-gradle-starter-ffi`)**:
   - Validação da integridade dos bindings gerados e serialização de tipos primitivos/complexos.

3. **Android (`android/app`)**:
   - Testes unitários com JUnit 5 + Mockito para ViewModels.
   - Previews no Jetpack Compose para validação visual estática.

4. **iOS (`ios/App`)**:
   - XCTest unitário para modelos `@Observable`.
   - Previews no SwiftUI Canvas.

5. **Desktop (`crates/mobile-gradle-starter-desktop`)**:
   - UI nativa renderizada em GPU com Rune (anomalyco/rune).
   - Reutiliza a crate `mobile-gradle-starter-core` diretamente em Rust, sem overhead de FFI.
   - Execução local: `cargo run --package mobile-gradle-starter-desktop`.

Execute no host:
```bash
cargo test --workspace
```
