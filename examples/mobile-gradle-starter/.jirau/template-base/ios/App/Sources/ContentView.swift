import SwiftUI
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
                    Text("🪵 mobile-gradle-starter")
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
