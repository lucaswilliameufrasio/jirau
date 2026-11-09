<div align="center">

English | [Português (Brasil)](README.pt-BR.md)

# 🪵 jirau

**Native cross-platform scaffold CLI: Rust + Android (Compose) + iOS (SwiftUI) + Desktop (Rune UI)**

*The speed of `flutter create` + `flutter doctor` + `flutter run` with the purity of 100% native UIs and high-performance desktop apps in Rust.*

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

## 💡 What is Jirau?

In the riverside houses of the Amazon, a **jirau** is a raised wooden platform built to support, organize and keep things safe. It is a sturdy, practical scaffold.

That metaphor defines the tool: **jirau** builds the solid structure your mobile app rests on and then **gets out of the way**. The generated project is not tied to the CLI, ships no proprietary runtime and builds normally with `cargo`, Gradle and Xcode.

---

## ⚡ Why jirau?

Traditional code-sharing approaches either force you to give up the native experience (Flutter/React Native) or impose heavy, opinionated architectures (Crux, Mobiler):

- 🦀 **The Rust core contains only logic:** No UI lifecycle management and no complex side-effect loops orchestrated inside Rust. Core unit tests run on the host with `cargo test` in milliseconds.
- 📱 **100% native, idiomatic mobile UI:**
  - **Android:** Plain Kotlin, Jetpack Compose, `ViewModel` and `StateFlow`.
  - **iOS:** Plain Swift (Swift 6 language mode), SwiftUI, `NavigationStack` and `@Observable` (iOS 17+, Apple Silicon only).
- 🖥️ **Native Rust desktop with Rune:**
  - GPU-accelerated desktop applications via [Rune UI](https://github.com/anomalyco/rune).
  - Reuses `<app>-core` directly in Rust, **with no FFI layer** and no conversion cost.
  - Available on Linux, Windows and macOS (Apple Silicon). *Note: Intel macOS (`x86_64`) is not supported.*
- 🔗 **Pure Mozilla UniFFI bridge:** No hand-written JNI in C/C++, no fragile intermediate layers. The Rust core is really linked on both platforms through UniFFI library mode, with types exported safely between Rust, Kotlin and Swift.
- 🚫 **Zero vendor lock-in:** `jirau` is a productivity accelerator, not a runtime framework. Your team keeps using Android Studio, Xcode and plain Cargo.

---

## 📂 Generated Project Structure

Running `jirau new my-app` creates a modular Cargo workspace with the native apps as sibling folders:

```text
my-app/
├── Cargo.toml               # Unified Cargo workspace
├── .mise.toml               # Pinned toolchains (JDK 25 LTS, Rust 1.99.1)
├── Makefile                 # Shortcuts: setup, doctor, test, android-ffi, ios-ffi, ...
├── crates/
│   ├── my-app-core/         # Pure business logic (no FFI, optional SQLite with rusqlite)
│   ├── my-app-ffi/          # Mozilla UniFFI definitions and proc-macros (mobile)
│   └── my-app-desktop/      # (Optional) Pure Rust desktop app with Rune UI
├── android/                 # (Optional) Native Jetpack Compose Android project (Gradle)
│   ├── gradlew              # Bundled Gradle wrapper (distribution pinned by SHA-256)
│   ├── settings.gradle.kts
│   ├── gradle.properties    # Tuned with configuration cache and parallel builds
│   ├── gradle/libs.versions.toml
│   └── app/                 # Jetpack Compose UI, ViewModels and StateFlow
├── ios/                     # (Optional) Native SwiftUI iOS project
│   ├── project.yml          # XcodeGen (declarative, no .pbxproj conflicts)
│   └── App/                 # SwiftUI UI with NavigationStack and @Observable
├── scripts/
│   ├── build-android-ffi.sh # cargo-ndk → jniLibs/ + Kotlin bindings
│   ├── build-ios-ffi.sh     # XCFramework + Swift bindings
│   └── setup-android-env.sh # Checks/installs Android prerequisites
├── docs/setup-android.md    # Android environment guide (Linux, headless or Android Studio)
├── .github/workflows/ci.yml # (Optional, --ci) Core, Android and iOS jobs
├── .jirau/template-base/    # Exact copy of the generated template (base for `jirau upgrade`)
├── jirau.toml               # App metadata (name, bundle id, platforms, storage, template version)
└── TESTING_STANDARD.md      # Testing standards and guidelines
```

### Architecture principles
1. **Strict separation between `core` and consumers:** The `core` contains only business rules in pure Rust. The desktop app consumes the `core` directly; mobile platforms consume it through `ffi`.
2. **Pure Rust desktop:** The `crates/<slug>-desktop` crate uses the [Rune](https://github.com/anomalyco/rune) framework, taking advantage of the GPU with modern rendering and high-performance reactivity.
3. **Transparent local persistence:** Optional embedded SQLite via `rusqlite` (`--sqlite`), with a `StorageEngine` in the Rust core and native bindings for Kotlin and Swift.
4. **Gradle is the Android build system:** Kotlin DSL with aggressive caching and a bundled wrapper. JetBrains Amper support was removed (its Gradle plugin mode was discontinued and standalone Amper has no stable 1.0); `--amper` now fails with an explanation.
5. **One build pipeline everywhere:** `scripts/build-ios-ffi.sh` and `scripts/build-android-ffi.sh` are the same scripts used by jirau, `make`, CI, Android Studio and Xcode. Gradle runs the Android script automatically in `preBuild` (choose ABIs with `-Pjirau.rustAbis=x86_64`).
6. **Generated artifacts stay out of version control:** Kotlin bindings (`android/app/build/generated/uniffi`), Swift bindings (`ios/Generated/`), `jniLibs/` and the XCFramework are regenerated automatically.
7. **Native mobile libraries:** Per-ABI `.so` files in `jniLibs/` on Android and a unified `XCFramework` (device + simulator) on iOS.
8. **XcodeGen:** The iOS project is described in `project.yml`, so its structure can be inspected and generated even from Linux. Simulator builds use ad-hoc signing (`CODE_SIGN_IDENTITY[sdk=iphonesimulator*] = "-"`) because the Keychain requires entitlements.
9. **Continuous evolution without breakage:** `jirau upgrade` performs a real 3-way merge using `.jirau/template-base/` as the base, preserving your custom code. Projects without a base get `<file>.jirau-new` side files for manual review.

---

## 🛠️ CLI Commands

A straightforward flow inspired by the Flutter ecosystem:

| Command | What it does | Flutter equivalent |
| :--- | :--- | :--- |
| `jirau new <name>` | Creates a project interactively or via flags (`--android`, `--ios`, `--desktop`, `--sqlite`, `--template starter\|vault`, `--theme`, `--plugin`, `--ci`, `--non-interactive`). | `flutter create` |
| `jirau add <platform>` | Adds a platform to an existing project (`jirau add desktop`, `android`, `ios`). | `flutter create . --platforms` |
| `jirau doctor` | Checks rustup, mobile targets, NDK, JDK, Android SDK, Xcode, XcodeGen and desktop dependencies (`--json` for CI, `--profile` for the Gradle build). | `flutter doctor` |
| `jirau run [android\|ios\|desktop]` | Builds, installs and launches the app on a device/simulator. Picks the single connected/booted device automatically or shows a selection menu; `-d <id>` skips the selection. Watch mode is on by default (`--no-watch` to disable). | `flutter run` |
| `jirau dev [android\|ios\|desktop]` | Fast development loop: rebuilds and redeploys when `crates/` changes. | `flutter run` (hot restart) |
| `jirau test [core\|ffi\|android\|ios\|desktop\|all]` | Unified test suite: Rust core, FFI lifecycle, Android instrumented tests on the connected device, XCTest on the iOS simulator, and desktop. | `flutter test` |
| `jirau bindings [android\|ios]` | Regenerates the Kotlin and Swift wrappers from the `-ffi` annotations. | `build_runner` |
| `jirau build [android\|ios\|desktop]` | Builds the final artifacts (multi-ABI APK, XCFramework or optimized desktop binary). | `flutter build` |
| `jirau upgrade [--force] [--dry-run]` | Applies template improvements via 3-way merge, preserving manual changes. | `flutter upgrade` |

**Language:** every command accepts `--lang en|pt-BR`. You can also set `JIRAU_LANG`; by default jirau follows the system locale. Messages, `--help` output and argument errors are all translated.

### Development loop optimizations
- **Automatic ABI detection:** Inspects the connected emulator or device (`adb shell getprop ro.product.cpu.abi`) and builds only the required target (`arm64-v8a` on a physical phone or `x86_64` on the emulator).
- **Single shared `target/`:** One Cargo workspace for Android, iOS and host tests, with optimized dependencies in the `dev` profile.

---

## 🧩 App Templates

| Template | What you get |
| :--- | :--- |
| `starter` (default) | A counter and a greeting computed by the Rust core, displayed by the native UIs. |
| `vault` | A password manager: SQLite storage with secrets encrypted with AES-256-GCM-SIV via `aws-lc-rs`. The 32-byte key is generated by the Rust core and stored in the iOS Keychain or wrapped by the Android Keystore. Legacy plaintext databases are migrated automatically. Titles, usernames and tags remain searchable plaintext metadata. |

---

## 📌 Pinned Technology Versions

All versions used by generated projects live in a single source of truth, [`src/templates/versions.rs`](src/templates/versions.rs). [`scripts/check-template-versions.sh`](scripts/check-template-versions.sh) compares them against the official registries and fails CI when any of them is outdated.

| Area | Version |
| :--- | :--- |
| Rust | 1.99.1 (edition 2024) |
| JDK | 25 (LTS) |
| Gradle | 9.9.0 (bundled wrapper, distribution SHA-256 verified) |
| Android Gradle Plugin | 9.5.1 |
| Kotlin | 2.4.21 |
| Android SDK | compileSdk/targetSdk 37, minSdk 26 |
| Compose BOM | 2026.09.00 |
| Android NDK | 30.0.16248370 |
| UniFFI | 0.32 |
| rusqlite | 0.41 |
| aws-lc-rs | 1.19 |
| Swift | Swift 6 language mode |
| iOS | 17+ deployment target |
| Xcode | 27 |

The current template version is **0.4.0**.

---

## 🚀 Installation

You can install the prebuilt binaries with the official scripts:

### Linux and macOS
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

## 📋 Prerequisites & Tools

`jirau doctor` diagnoses and helps you install what is needed:

- **Rust:** `rustup` (the generated `.mise.toml` pins Rust 1.99.1).
- **Rust mobile targets:**
  - Android: `aarch64-linux-android`, `x86_64-linux-android`
  - iOS: `aarch64-apple-ios`, `aarch64-apple-ios-sim`
- **Android:** Android SDK (platform 37), NDK 30.0.16248370, JDK 25 and `cargo-ndk`. See [`docs/setup-android.md`](docs/setup-android.md).
- **iOS:** macOS on Apple Silicon with Xcode 27 and `xcodegen` (`brew install xcodegen`). *(Intel macOS is not supported.)*
- **Desktop (Rune UI):**
  - **Linux:** Window/GPU development libraries (`pkg-config`, `libxkbcommon-dev`, `libfontconfig1-dev` on Debian/Ubuntu).
  - **macOS:** Apple Silicon (`aarch64-apple-darwin`) with native Metal.
  - **Windows:** Native DirectX 12 / Vulkan support via MSVC.

---

## 🌟 Example Projects

The [`examples/`](examples/) folder contains projects regenerated from the templates to demonstrate each combination:

- **[`secure-vault`](examples/secure-vault/)**: A real, working **password manager / local vault** (the `vault` template): encrypted SQLite storage, real-time search, a cryptographic password generator and entropy meter in the Rust core, plus Jetpack Compose, SwiftUI and Rune desktop UIs. The `vault` template is synced from this example with `scripts/sync-vault-template.py`.
- **[`all-platforms-showcase`](examples/all-platforms-showcase/)**: An app with **every platform enabled** (Android Jetpack Compose + iOS SwiftUI + Desktop Rune UI), embedded SQLite (`rusqlite`), the extended Material 3 theme and FFI lifecycle tests.
- **[`mobile-gradle-starter`](examples/mobile-gradle-starter/)**: A mobile-only setup (Android + iOS) with standard Gradle (Kotlin DSL) and reactive in-memory state.
- **[`desktop-first-sqlite`](examples/desktop-first-sqlite/)**: A native Rust desktop app using the SQLite `StorageEngine` directly, with no FFI bridge.

Validated end to end on the iOS simulator (iPhone 18 Pro, Xcode 27.2) and the Android emulator (API 36, x86_64): `run`, tests, persistence and encryption.

See [`examples/README.md`](examples/README.md) for detailed instructions on running them.

---

## 🗺️ Roadmap

- [x] **Phase 0: Concept & Architecture** ([`docs/development-plan.md`](docs/development-plan.md))
- [x] **Phase 1: Template Extraction & Consolidation** (UniFFI, Compose, SwiftUI, XcodeGen `project.yml`, Gradle and Cargo caches)
- [x] **Phase 2: Guided CLI Core (`new` and `doctor`)** (step-by-step wizard with `inquire`, environment diagnostics with exact fix suggestions and `--json` support)
- [x] **Phase 3: Android Hot Loop (`dev` & `bindings`)** (automatic ABI detection via `adb`, incremental single-ABI builds via `cargo-ndk`, file watcher with `notify` and Kotlin bindings)
- [x] **Phase 4: iOS Support (`dev`, `bindings` & `build`)** (declarative XcodeGen integration, XCFramework and simulators)
- [x] **Phase 5: Dogfooding & Test Suite** (unit tests and an end-to-end integration test that generates temporary apps and runs `cargo test`)
- [x] **Phase 6: Release Automation & Distribution** (`cargo-dist` pipeline, automatic changelog via `git-cliff`, binaries for Linux, macOS and Windows in v0.1.0)
- [x] **Phase 7: Persistence & Upgrades (v0.2.0)** (embedded SQLite in the core via `rusqlite` and the `jirau upgrade` command; the JetBrains Amper support added in this phase was later removed)
- [x] **Phase 8: Themes, Plugins & End-to-End FFI Tests (v0.3.0)** (design tokens for Compose/SwiftUI, multithreaded native lifecycle tests on the host and the `examples/` showcase)
- [x] **Phase 9: Real Native Linking & Devices (template v0.4.0)** (Rust core linked via UniFFI library mode on both platforms, `jirau run` and on-device `jirau test`, `vault` template with encryption, pinned versions checked against official registries, real 3-way `upgrade`, English/Portuguese CLI)

---

## 📄 License

Distributed under the MIT or Apache 2.0 license (`MIT OR Apache-2.0`).
