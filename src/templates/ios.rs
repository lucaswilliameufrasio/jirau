use super::{TemplateContext, versions};

pub fn project_yml(ctx: &TemplateContext) -> String {
    format!(
        r#"name: {pascal_name}
options:
  bundleIdPrefix: {package_id}
  deploymentTarget:
    iOS: "{ios_target}"
  xcodeVersion: "{xcode}"

settings:
  base:
    SWIFT_VERSION: "{swift}"
    PRODUCT_BUNDLE_IDENTIFIER: {package_id}
    MARKETING_VERSION: "1.0.0"
    CURRENT_PROJECT_VERSION: "1"
    # Simulador: assinatura ad-hoc ("Sign to Run Locally"), sem time de
    # desenvolvimento. Sem assinatura o app não tem entitlements e o Keychain
    # falha com errSecMissingEntitlement (-34018).
    "CODE_SIGN_IDENTITY[sdk=iphonesimulator*]": "-"

targets:
  {pascal_name}:
    type: application
    platform: iOS
    sources:
      - path: App/Sources
      # Bindings Swift gerados pelo UniFFI (scripts/build-ios-ffi.sh)
      - path: Generated
        optional: true
    settings:
      base:
        GENERATE_INFOPLIST_FILE: YES
        INFOPLIST_KEY_UILaunchScreen_Generation: YES
        INFOPLIST_KEY_UISupportedInterfaceOrientations: "UIInterfaceOrientationPortrait"
        ENABLE_PREVIEWS: YES
    dependencies:
      # Núcleo Rust estático (scripts/build-ios-ffi.sh). Biblioteca estática: não embutir.
      - framework: Frameworks/{rust_name}_ffi.xcframework
        embed: false
    scheme:
      testTargets:
        - {pascal_name}Tests

  {pascal_name}Tests:
    type: bundle.unit-test
    platform: iOS
    sources:
      - path: App/Tests
    settings:
      base:
        GENERATE_INFOPLIST_FILE: YES
        PRODUCT_BUNDLE_IDENTIFIER: {package_id}.tests
    dependencies:
      - target: {pascal_name}
"#,
        pascal_name = ctx.pascal_name,
        package_id = ctx.package_id,
        rust_name = ctx.rust_name,
        ios_target = versions::IOS_DEPLOYMENT_TARGET,
        xcode = versions::XCODE_VERSION,
        swift = versions::SWIFT_LANGUAGE_VERSION,
    )
}

/// Script autocontido que compila o núcleo Rust para iOS, gera os bindings
/// Swift via UniFFI (library mode) e monta o XCFramework consumido pelo Xcode.
/// É a única fonte da verdade do pipeline: `jirau`, `make` e CI chamam este script.
pub fn build_ios_ffi_sh(ctx: &TemplateContext) -> String {
    r#"#!/usr/bin/env bash
# ==============================================================================
# Compila o núcleo Rust para iOS, gera os bindings Swift (UniFFI) e monta o
# XCFramework em ios/Frameworks. Gerado via jirau — pode ser usado sem a CLI.
#
# Uso: scripts/build-ios-ffi.sh [debug|release]
#   IOS_TARGETS="aarch64-apple-ios-sim aarch64-apple-ios"  sobrescreve os alvos
# ==============================================================================
set -euo pipefail

PROFILE="${1:-debug}"
CRATE="__SLUG__-ffi"
LIB="__RUST_NAME___ffi"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

case "$PROFILE" in
  debug)
    RELEASE_FLAG=""
    DEFAULT_TARGETS="aarch64-apple-ios-sim"
    ;;
  release)
    RELEASE_FLAG="--release"
    DEFAULT_TARGETS="aarch64-apple-ios aarch64-apple-ios-sim"
    ;;
  *)
    echo "Perfil inválido: $PROFILE (use 'debug' ou 'release')" >&2
    exit 2
    ;;
esac

read -r -a TARGETS <<< "${IOS_TARGETS:-$DEFAULT_TARGETS}"
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-__IOS_TARGET__}"

if command -v rustup >/dev/null 2>&1; then
  INSTALLED="$(rustup target list --installed)"
  for t in "${TARGETS[@]}"; do
    if ! grep -qx "$t" <<< "$INSTALLED"; then
      echo "➜ Instalando target Rust $t..."
      rustup target add "$t"
    fi
  done
fi

# 1. Biblioteca estática por alvo iOS
for t in "${TARGETS[@]}"; do
  echo "➜ cargo build ($t, $PROFILE)"
  cargo build -p "$CRATE" --lib --target "$t" $RELEASE_FLAG
done

# 2. Bindings Swift: library mode lê os metadados UniFFI de um build de host
#    em debug (o perfil release usa strip, que remove esses metadados).
echo "➜ Gerando bindings Swift via UniFFI"
cargo build -p "$CRATE" --lib
WORK="target/jirau-ios/$PROFILE"
rm -rf "$WORK"
mkdir -p "$WORK/swift" "$WORK/headers"
cargo run -q -p "$CRATE" --bin uniffi-bindgen -- generate   --library "target/debug/lib${LIB}.dylib"   --language swift   --out-dir "$WORK/swift"

cp "$WORK"/swift/*.h "$WORK/headers/"
cat "$WORK"/swift/*.modulemap > "$WORK/headers/module.modulemap"

mkdir -p ios/Generated ios/Frameworks
find ios/Generated -name '*.swift' -delete
cp "$WORK"/swift/*.swift ios/Generated/

# 3. XCFramework (uma fatia por plataforma)
XCF="ios/Frameworks/${LIB}.xcframework"
rm -rf "$XCF"
ARGS=()
for t in "${TARGETS[@]}"; do
  ARGS+=(-library "target/$t/$PROFILE/lib${LIB}.a" -headers "$WORK/headers")
done
xcodebuild -create-xcframework "${ARGS[@]}" -output "$XCF" >/dev/null

echo "✅ Núcleo Rust pronto para iOS: $XCF + ios/Generated/"
"#
    .replace("__SLUG__", &ctx.slug)
    .replace("__RUST_NAME__", &ctx.rust_name)
    .replace("__IOS_TARGET__", versions::IOS_DEPLOYMENT_TARGET)
}

pub fn app_swift(ctx: &TemplateContext) -> String {
    format!(
        r#"import SwiftUI

@main
struct {pascal_name}App: App {{
    var body: some Scene {{
        WindowGroup {{
            ContentView()
        }}
    }}
}}
"#,
        pascal_name = ctx.pascal_name
    )
}

pub fn content_view_swift(ctx: &TemplateContext) -> String {
    r##"import SwiftUI
import Observation

// `AppEngine`, `NativeError` e `nativeGreet` vêm do núcleo Rust via UniFFI
// (ios/Generated, gerado por scripts/build-ios-ffi.sh).

@Observable
final class MainViewModel {
    private let engine: AppEngine
    private(set) var counter: Int32
    private(set) var status: String

    init(engine: AppEngine = AppEngine()) {
        self.engine = engine
        self.counter = engine.getState().counter
        self.status = nativeGreet(name: "iOS")
    }

    func increment() {
        counter = engine.increment().counter
        status = "Incrementado pelo StateEngine em Rust"
    }

    func decrement() {
        do {
            counter = try engine.decrement().counter
            status = "Decrementado pelo StateEngine em Rust"
        } catch NativeError.NegativeValue {
            status = "Rust recusou: o contador não pode ficar negativo"
        } catch {
            status = "Erro do núcleo Rust: \(error)"
        }
    }
}

struct ContentView: View {
    @State private var viewModel = MainViewModel()

    var body: some View {
        NavigationStack {
            VStack(spacing: 24) {
                VStack(spacing: 8) {
                    Text("🪵 __NAME__")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                        .multilineTextAlignment(.center)
                    Text("UI Nativa iOS em SwiftUI")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }

                GroupBox {
                    VStack(spacing: 12) {
                        Text("Contador (Rust Core):")
                            .font(.headline)
                            .foregroundStyle(.secondary)
                        Text("\(viewModel.counter)")
                            .font(.system(size: 64, weight: .bold, design: .rounded))
                            .foregroundStyle(.tint)
                            .accessibilityIdentifier("counter")
                        Text(viewModel.status)
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                            .accessibilityIdentifier("status")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                }

                HStack(spacing: 16) {
                    Button(role: .destructive) {
                        viewModel.decrement()
                    } label: {
                        Label("Menos", systemImage: "minus.circle")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)
                    .accessibilityIdentifier("decrement")

                    Button {
                        viewModel.increment()
                    } label: {
                        Label("Mais", systemImage: "plus.circle.fill")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    .accessibilityIdentifier("increment")
                }
                .padding(.horizontal)

                Spacer()
            }
            .padding()
        }
    }
}

#Preview {
    ContentView()
}
"##
    .replace("__NAME__", &ctx.name)
}

pub fn theme_swift() -> String {
    r#"import SwiftUI

public enum JirauTheme {
    public static let primaryColor = Color(red: 0.12, green: 0.34, blue: 0.19)
    public static let accentColor = Color(red: 0.16, green: 0.32, blue: 0.75)
    public static let earthColor = Color(red: 0.82, green: 0.49, blue: 0.18)
    public static let cornerRadius: CGFloat = 14.0
}

public struct JirauCardModifier: ViewModifier {
    public func body(content: Content) -> some View {
        content
            .padding()
            .background(Color(.secondarySystemBackground))
            .clipShape(RoundedRectangle(cornerRadius: JirauTheme.cornerRadius, style: .continuous))
            .shadow(color: Color.black.opacity(0.04), radius: 6, x: 0, y: 3)
    }
}

public extension View {
    func jirauCardStyle() -> some View {
        modifier(JirauCardModifier())
    }
}
"#
    .to_string()
}

pub fn lifecycle_tests_swift(ctx: &TemplateContext) -> String {
    r##"import XCTest
@testable import __PASCAL__

/// Testes do ViewModel contra o núcleo Rust real (AppEngine via UniFFI).
final class NativeLifecycleTests: XCTestCase {
    func testViewModelStartsFromRustState() {
        let vm = MainViewModel()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("Rust"), "saudação deve vir do núcleo Rust: \(vm.status)")
    }

    func testStateTransitionsGoThroughRust() {
        let engine = AppEngine()
        let vm = MainViewModel(engine: engine)
        vm.increment()
        vm.increment()
        XCTAssertEqual(vm.counter, 2)
        XCTAssertEqual(engine.getState().counter, 2, "o estado vive no Rust")

        vm.decrement()
        XCTAssertEqual(vm.counter, 1)
    }

    func testRustRejectsNegativeCounter() {
        let vm = MainViewModel()
        vm.decrement()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("negativo"))
    }
}
"##
    .replace("__PASCAL__", &ctx.pascal_name)
}
