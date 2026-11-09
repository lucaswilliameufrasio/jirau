//! Única fonte das versões fixadas nos projetos gerados pelo jirau.
//!
//! Todas foram conferidas nos registros oficiais (Gradle, Google Maven, Maven
//! Central, crates.io, Adoptium, GitHub Releases) em `VERIFIED_ON`. Para
//! revalidar, rode `scripts/check-template-versions.sh` na raiz do repositório
//! do jirau: ele falha quando alguma versão daqui deixou de ser a estável mais
//! recente.

/// Data da última conferência com os registros oficiais.
pub const VERIFIED_ON: &str = "2026-11-09";

// --- Toolchains ---------------------------------------------------------------
pub const RUST: &str = "1.99.1";
pub const RUST_EDITION: &str = "2024";
/// JDK LTS mais recente (Adoptium `most_recent_lts`).
pub const JAVA_LTS: &str = "25";

// --- Android / Gradle -----------------------------------------------------------
pub const GRADLE: &str = "9.9.0";
/// SHA-256 de `gradle-<GRADLE>-bin.zip` (services.gradle.org), validado pelo wrapper.
pub const GRADLE_DIST_SHA256: &str =
    "0b00b0ffe07c4d4789c12d4935a44a7057d55774b7540bd4a3f76f336deb177a";
pub const AGP: &str = "9.5.1";
pub const KOTLIN: &str = "2.4.21";
pub const ANDROID_COMPILE_SDK: u32 = 37;
pub const ANDROID_TARGET_SDK: u32 = 37;
pub const ANDROID_MIN_SDK: u32 = 26;
/// Canal estável do repositório do SDK (dl.google.com/android/repository).
pub const ANDROID_BUILD_TOOLS: &str = "37.0.0";
pub const ANDROID_NDK: &str = "30.0.16248370";
/// Bytecode gerado para o Android (D8 aceita até Java 21).
pub const ANDROID_JVM_TARGET: &str = "21";
pub const COMPOSE_BOM: &str = "2026.09.00";
pub const ANDROIDX_CORE_KTX: &str = "1.19.1";
pub const ANDROIDX_LIFECYCLE: &str = "2.11.0";
pub const ANDROIDX_ACTIVITY_COMPOSE: &str = "1.13.0";
pub const ANDROIDX_TEST_EXT_JUNIT: &str = "1.3.0";
pub const ANDROIDX_TEST_RUNNER: &str = "1.7.0";
pub const JUNIT4: &str = "4.13.2";
pub const JNA: &str = "5.19.1";
pub const KOTLINX_COROUTINES: &str = "1.11.0";

// --- iOS ------------------------------------------------------------------------
pub const SWIFT_LANGUAGE_VERSION: &str = "6.0";
pub const XCODE_VERSION: &str = "27.0";
/// Alcance de aparelhos (não é "versão da tecnologia"): iOS 17+ cobre a grande
/// maioria dos iPhones em uso e é o mínimo do `@Observable`.
pub const IOS_DEPLOYMENT_TARGET: &str = "17.0";

// --- Crates Rust (requisitos semver do Cargo) -------------------------------------
pub const UNIFFI: &str = "0.32";
pub const RUSQLITE: &str = "0.41";
pub const SERDE: &str = "1";
pub const SERDE_JSON: &str = "1";
pub const THISERROR: &str = "2";
pub const UUID: &str = "1.29";
/// Criptografia e CSPRNG do cofre (AWS-LC, FIPS-capable).
pub const AWS_LC_RS: &str = "1.19";

// --- GitHub Actions ----------------------------------------------------------------
pub const ACTION_CHECKOUT: &str = "actions/checkout@v7";
pub const ACTION_SETUP_JAVA: &str = "actions/setup-java@v6";
pub const ACTION_GRADLE: &str = "gradle/actions/setup-gradle@v6";
pub const ACTION_RUST_CACHE: &str = "Swatinem/rust-cache@v2";
