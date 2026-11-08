import SwiftUI
import Observation

@Observable
class MainViewModel {
    var counter: Int = 0
    var status: String = "Núcleo Rust conectado"

    func increment() {
        counter += 1
        status = "Incrementado via Rust StateEngine"
    }

    func decrement() {
        if counter > 0 {
            counter -= 1
            status = "Decrementado via Rust StateEngine"
        } else {
            status = "Erro validado no Rust: valor não pode ser negativo"
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
                        Text(viewModel.status)
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                }

                HStack(spacing: 16) {
                    Button(role: .destructive) {
                        viewModel.decrement()
                    } label: {
                        Label("Decrementar", systemImage: "minus.circle")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)

                    Button {
                        viewModel.increment()
                    } label: {
                        Label("Incrementar", systemImage: "plus.circle.fill")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
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
