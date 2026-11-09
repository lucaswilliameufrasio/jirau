import SwiftUI

struct VaultItemModel: Identifiable {
    let id: String
    var title: String
    var category: String
    var username: String
    var secretPayload: String
    var isFavorite: Bool
}

@Observable
class VaultViewModel {
    var items: [VaultItemModel] = [
        VaultItemModel(id: "1", title: "GitHub Personal Token", category: "API Key", username: "lucas@dev.io", secretPayload: "ghp_secureKeyExample999", isFavorite: true),
        VaultItemModel(id: "2", title: "AWS Production Console", category: "Login", username: "admin@corp.cloud", secretPayload: "P@ssw0rdProd#2026", isFavorite: true),
        VaultItemModel(id: "3", title: "Chave Mestra de Criptografia", category: "Nota", username: "", secretPayload: "AES256-GCM-PayloadKey-Local", isFavorite: false)
    ]
    var searchQuery: String = ""
    var generatedPassword: String = "S3cur3#P@ss-2026"
    var healthScore: Int = 94

    var filteredItems: [VaultItemModel] {
        if searchQuery.isEmpty {
            return items
        }
        return items.filter {
            $0.title.localizedCaseInsensitiveContains(searchQuery) ||
            $0.category.localizedCaseInsensitiveContains(searchQuery)
        }
    }

    func generateNewPassword() {
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%&*"
        let pwd = String((0..<18).compactMap { _ in chars.randomElement() })
        self.generatedPassword = pwd
    }

    func toggleFavorite(id: String) {
        if let idx = items.firstIndex(where: { $0.id == id }) {
            items[idx].isFavorite.toggle()
        }
    }
}

struct ContentView: View {
    @State private var vm = VaultViewModel()

    var body: some View {
        NavigationStack {
            List {
                Section(header: Text("Métricas & Gerador")) {
                    HStack {
                        Text("Saúde do Cofre:")
                        Spacer()
                        Text("\(vm.healthScore)% Seguro")
                            .bold()
                            .foregroundStyle(.green)
                    }
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Senha Segura Gerada (Rust Core):")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Text(vm.generatedPassword)
                            .font(.system(.body, design: .monospaced))
                            .bold()
                            .foregroundStyle(.blue)
                        Button(action: { vm.generateNewPassword() }) {
                            Label("Gerar Nova Senha", systemImage: "arrow.clockwise")
                        }
                    }
                }

                Section(header: Text("Credenciais Armazenadas (SQLite)")) {
                    ForEach(vm.filteredItems) { item in
                        HStack {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(item.title)
                                    .font(.headline)
                                Text("\(item.category) • \(item.username)")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                Text("••••••••••••••••")
                                    .font(.caption)
                                    .foregroundStyle(.blue)
                            }
                            Spacer()
                            Button(action: { vm.toggleFavorite(id: item.id) }) {
                                Image(systemName: item.isFavorite ? "star.fill" : "star")
                                    .foregroundStyle(item.isFavorite ? .yellow : .gray)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
            .navigationTitle("🔐 Secure Vault Vault")
            .searchable(text: $vm.searchQuery, prompt: "Buscar credenciais...")
        }
    }
}
