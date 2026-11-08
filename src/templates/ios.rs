use super::TemplateContext;

pub fn project_yml(ctx: &TemplateContext) -> String {
    format!(
        r#"name: {pascal_name}
options:
  bundleIdPrefix: {package_id}
  deploymentTarget:
    iOS: "17.0"
  xcodeVersion: "16.0"

settings:
  base:
    SWIFT_VERSION: "5.10"
    PRODUCT_BUNDLE_IDENTIFIER: {package_id}
    MARKETING_VERSION: "1.0.0"
    CURRENT_PROJECT_VERSION: "1"

targets:
  {pascal_name}:
    type: application
    platform: iOS
    sources:
      - path: App/Sources
      - path: Generated
        optional: true
    settings:
      base:
        INFOPLIST_KEY_UILaunchScreen_Generation: YES
        INFOPLIST_KEY_UISupportedInterfaceOrientations: "UIInterfaceOrientationPortrait"
        ENABLE_PREVIEWS: YES
    frameworks:
      - framework: Frameworks/{rust_name}_ffi.xcframework
        optional: true
"#,
        pascal_name = ctx.pascal_name,
        package_id = ctx.package_id,
        rust_name = ctx.rust_name
    )
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
    format!(
        r#"import SwiftUI
import Observation

@Observable
class MainViewModel {{
    var counter: Int = 0
    var status: String = "Núcleo Rust conectado"

    func increment() {{
        counter += 1
        status = "Incrementado via Rust StateEngine"
    }}

    func decrement() {{
        if counter > 0 {{
            counter -= 1
            status = "Decrementado via Rust StateEngine"
        }} else {{
            status = "Erro validado no Rust: valor não pode ser negativo"
        }}
    }}
}}

struct ContentView: View {{
    @State private var viewModel = MainViewModel()

    var body: some View {{
        NavigationStack {{
            VStack(spacing: 24) {{
                VStack(spacing: 8) {{
                    Text("🪵 {name}")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("UI Nativa iOS em SwiftUI")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }}

                GroupBox {{
                    VStack(spacing: 12) {{
                        Text("Contador (Rust Core):")
                            .font(.headline)
                            .foregroundStyle(.secondary)
                        Text("\(viewModel.counter)")
                            .font(.system(size: 64, weight: .bold, design: .rounded))
                            .foregroundStyle(.tint)
                        Text(viewModel.status)
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                    }}
                    .frame(maxWidth: .infinity)
                    .padding()
                }}

                HStack(spacing: 16) {{
                    Button(role: .destructive) {{
                        viewModel.decrement()
                    }} label: {{
                        Label("Decrementar", systemImage: "minus.circle")
                            .frame(maxWidth: .infinity)
                    }}
                    .buttonStyle(.bordered)
                    .controlSize(.large)

                    Button {{
                        viewModel.increment()
                    }} label: {{
                        Label("Incrementar", systemImage: "plus.circle.fill")
                            .frame(maxWidth: .infinity)
                    }}
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                }}
                .padding(.horizontal)

                Spacer()
            }}
            .padding()
        }}
    }}
}}

#Preview {{
    ContentView()
}}
"#,
        name = ctx.name
    )
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
    format!(
        r#"import XCTest
@testable import {pascal_name}

final class NativeLifecycleTests: XCTestCase {{
    func testViewModelInitialState() {{
        let vm = MainViewModel()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("conectado"))
    }}

    func testViewModelStateTransitions() {{
        let vm = MainViewModel()
        vm.increment()
        XCTAssertEqual(vm.counter, 1)

        vm.decrement()
        XCTAssertEqual(vm.counter, 0)

        // Limite inferior
        vm.decrement()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("não pode ser negativo"))
    }}
}}
"#,
        pascal_name = ctx.pascal_name
    )
}
