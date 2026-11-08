use super::TemplateContext;

pub fn root_cargo_toml(ctx: &TemplateContext) -> String {
    format!(
        r#"[workspace]
members = [
    "crates/{slug}-core",
    "crates/{slug}-ffi",
]
resolver = "2"

[profile.dev.package."*"]
opt-level = 2

[profile.release]
opt-level = 3
lto = "thin"
strip = true
"#,
        slug = ctx.slug
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

pub fn testing_standard_md(ctx: &TemplateContext) -> String {
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

Execute no host:
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

    format!(
        r#"[app]
name = "{name}"
package_id = "{package_id}"
version = "0.1.0"
platforms = [{platforms}]

[dev]
default_android_abi = "arm64-v8a"
default_ios_simulator = "iPhone 16"
use_sccache = true

[meta]
template_version = "0.1.0"
"#,
        name = ctx.name,
        package_id = ctx.package_id,
        platforms = platforms_str
    )
}
