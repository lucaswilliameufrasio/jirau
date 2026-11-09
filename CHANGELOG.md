# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-11-09

### Features
- **Smart Device Selection in `jirau run [platform]`**: Automatically runs on a single connected Android device or booted iOS simulator; opens interactive picker when multiple are present; displays helpful setup guidance when none is available.
- **Unified Test Runner `jirau test [target]`**: Orchestrates test execution across Rust Core, FFI lifecycle integration tests, Android JUnit 5 unit tests, iOS XCTest, and Desktop Rune compilation.
- **Production-Ready "Secure Vault" Sample App**: Full-featured password manager showcasing SQLite with relational schema & indexes, search & categorization, cryptographic password generator with strength evaluation, Rune GPU Desktop UI, Jetpack Compose Material 3, and SwiftUI with `@Observable`.
- **Environment & Automation Improvements**: Headless Android SDK setup documentation & automated setup script, `.mise.toml` pinned versions, root Makefile automation, and enhanced `jirau doctor` fallbacks.
- **Material 3 Extended Themes & Plugins**: Design token generators for Compose and SwiftUI, plus `logger` plugin integration.
- **CI/CD Template Generation**: `--ci` flag adds ready-to-run GitHub Actions workflow (`.github/workflows/ci.yml`).

## [0.2.0] - 2026-11-08

### Features
- **Desktop Support (Rune UI)**: Native desktop applications rendered on GPU via `anomalyco/rune`, consuming the Rust core directly without FFI overhead.
- **SQLite Persistence (`--sqlite`)**: Embedded relational database in core via `rusqlite` with typed queries and native bindings.
- **JetBrains Amper Support (`--amper`)**: Declarative YAML configuration for Android projects.
- **Template Upgrade Engine (`jirau upgrade`)**: 3-way merge algorithm to synchronize projects with new template versions while preserving developer customizations.
- **Reference Showcase Projects**: Added `all-platforms-showcase`, `mobile-gradle-starter`, and `desktop-first-sqlite` in `examples/`.

## [0.1.0] - 2026-11-07

### Features
- Initial release of jirau CLI
- Interactive guided workflow with step-by-step prompts for `new`, `doctor`, `dev`, `bindings`, `build`
- Complete native scaffolding for Rust core, UniFFI bridge, Android (Jetpack Compose), and iOS (SwiftUI + XcodeGen)
- Diagnostic doctor engine with actionable hints and `--json` support
- Fast build optimizations with ABI auto-detection and file watcher

### Documentation
- Add comprehensive README with architecture, workflow comparison, and philosophy
- Add development plan and testing guidelines
