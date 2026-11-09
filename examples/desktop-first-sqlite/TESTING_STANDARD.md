# Testing Guidelines — desktop-first-sqlite

This repository follows a fast, layered testing methodology:

1. **Rust core (`crates/desktop-first-sqlite-core`)**:
   - Pure unit tests run directly on the host via `cargo test`.
   - They do not depend on an emulator, simulator, Android NDK or UniFFI.
   - Execution target: < 500 ms.

2. **FFI layer (`crates/desktop-first-sqlite-ffi`)**:
   - Validates the integrity of the generated bindings and the serialization of primitive/complex types.

3. **Android (`android/app`)**:
   - Unit tests with JUnit 5 + Mockito for ViewModels.
   - Jetpack Compose previews for static visual validation.

4. **iOS (`ios/App`)**:
   - XCTest unit tests for `@Observable` models.
   - SwiftUI Canvas previews.

5. **Desktop (`crates/desktop-first-sqlite-desktop`)**:
   - Native GPU-rendered UI with Rune (anomalyco/rune).
   - Reuses the `desktop-first-sqlite-core` crate directly in Rust, with no FFI overhead.
   - Run locally: `cargo run --package desktop-first-sqlite-desktop`.

Run on the host:
```bash
cargo test --workspace
```
