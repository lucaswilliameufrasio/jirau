import SwiftUI
import Observation
import UIKit
import Security

// =============================================================================
// Integração com o núcleo Rust (UniFFI)
// Tipos `Native*`, `NativeVaultEngine`, `generateSecurePassword` e
// `evaluatePassword` vêm de ios/Generated (scripts/build-ios-ffi.sh).
// =============================================================================

extension NativeCategory: CaseIterable, Identifiable {
    public static var allCases: [NativeCategory] { [.login, .note, .apiKey, .creditCard] }
    public var id: Self { self }

    var displayName: String {
        switch self {
        case .login: return "Login"
        case .note: return "Nota"
        case .apiKey: return "Chave de API"
        case .creditCard: return "Cartão"
        }
    }

    var systemImage: String {
        switch self {
        case .login: return "person.badge.key"
        case .note: return "note.text"
        case .apiKey: return "key.horizontal"
        case .creditCard: return "creditcard"
        }
    }
}

extension NativeVaultItem: Identifiable {}

func describe(_ error: Error) -> String {
    if let vaultError = error as? NativeVaultError {
        switch vaultError {
        case .InvalidInput:
            return "Título e segredo são obrigatórios."
        case .NotFound(let id):
            return "Item não encontrado (\(id))."
        case .Database(let detail):
            return "Erro no banco de dados: \(detail)"
        case .InvalidKey:
            return "Chave do cofre inválida."
        case .WrongKeyOrCorrupted:
            return "A chave do Keychain não abre este cofre (chave trocada ou dados corrompidos)."
        }
    }
    return error.localizedDescription
}

// =============================================================================
// Chave do cofre no Keychain
// O Rust cifra cada segredo com AES-256-GCM-SIV (aws-lc-rs); a chave de 32 bytes é
// gerada pelo próprio núcleo e fica só no Keychain (nunca no SQLite).
// =============================================================================

struct KeychainError: LocalizedError {
    let status: OSStatus

    var errorDescription: String? {
        let detail = SecCopyErrorMessageString(status, nil) as String? ?? "erro desconhecido"
        return "Keychain (\(status)): \(detail)"
    }
}

enum VaultKeyStore {
    private static let service = (Bundle.main.bundleIdentifier ?? "vault") + ".vault-key"
    private static let account = "vault-master-key"

    private static var baseQuery: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
    }

    /// Lê a chave do cofre; na primeira execução gera uma nova e a guarda.
    static func loadOrCreate() throws -> Data {
        if let key = try load() {
            return key
        }
        let key = generateVaultKey()
        var query = baseQuery
        query[kSecValueData as String] = key
        // Só neste aparelho e só depois do primeiro desbloqueio: não vai para
        // backups nem para outros dispositivos.
        query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainError(status: status) }
        return key
    }

    private static func load() throws -> Data? {
        var query = baseQuery
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        switch status {
        case errSecSuccess:
            return result as? Data
        case errSecItemNotFound:
            return nil
        default:
            throw KeychainError(status: status)
        }
    }
}

func strengthColor(_ score: UInt32) -> Color {
    switch score {
    case 70...: return .green
    case 40..<70: return .orange
    default: return .red
    }
}

// =============================================================================
// ViewModel: toda regra de negócio e persistência acontece no Rust.
// =============================================================================

@Observable
final class VaultViewModel {
    private let engine: NativeVaultEngine

    private(set) var items: [NativeVaultItem] = []
    private(set) var stats: NativeVaultStats?
    var searchQuery = ""
    var categoryFilter: NativeCategory?
    var passwordLength: Double = 20
    private(set) var generatedPassword = ""
    private(set) var generatedEvaluation: NativePasswordEvaluation?
    var errorMessage: String?

    init(engine: NativeVaultEngine) {
        self.engine = engine
        regeneratePassword()
        reload()
    }

    /// Cofre persistido em SQLite em Application Support, com a chave no Keychain.
    static func persistent() -> VaultViewModel {
        do {
            let dir = try FileManager.default.url(
                for: .applicationSupportDirectory,
                in: .userDomainMask,
                appropriateFor: nil,
                create: true
            )
            let path = dir.appendingPathComponent("vault.sqlite3").path
            let key = try VaultKeyStore.loadOrCreate()
            return VaultViewModel(engine: try NativeVaultEngine.open(path: path, key: key))
        } catch {
            // Sem disco ou sem Keychain o app segue com um cofre temporário:
            // nada é gravado em disco sem criptografia.
            let vm = VaultViewModel(engine: try! NativeVaultEngine.inMemory())
            vm.errorMessage = "Cofre em disco indisponível (\(describe(error))). Usando cofre temporário."
            return vm
        }
    }

    func reload() {
        perform {
            let query = searchQuery.trimmingCharacters(in: .whitespaces)
            items = try engine.listItems(category: categoryFilter, query: query.isEmpty ? nil : query)
            stats = try engine.getStats()
        }
    }

    func item(id: String) -> NativeVaultItem? {
        try? engine.getItem(id: id)
    }

    @discardableResult
    func save(
        id: String?,
        title: String,
        category: NativeCategory,
        username: String,
        secret: String,
        tags: [String],
        isFavorite: Bool
    ) throws -> NativeVaultItem {
        let trimmedUser = username.trimmingCharacters(in: .whitespaces)
        let item = try engine.saveItem(input: NativeVaultItemInput(
            id: id,
            title: title,
            category: category,
            username: trimmedUser.isEmpty ? nil : trimmedUser,
            secretPayload: secret,
            tags: tags,
            isFavorite: isFavorite
        ))
        reload()
        return item
    }

    func delete(id: String) {
        perform { _ = try engine.deleteItem(id: id) }
        reload()
    }

    func toggleFavorite(id: String) {
        perform { _ = try engine.toggleFavorite(id: id) }
        reload()
    }

    func regeneratePassword() {
        let options = NativePasswordOptions(
            length: UInt32(passwordLength),
            includeUppercase: true,
            includeLowercase: true,
            includeNumbers: true,
            includeSymbols: true
        )
        generatedPassword = generateSecurePassword(options: options)
        generatedEvaluation = evaluatePassword(password: generatedPassword)
    }

    func evaluate(_ password: String) -> NativePasswordEvaluation {
        evaluatePassword(password: password)
    }

    func loadSampleItems() {
        perform {
            _ = try engine.saveItem(input: NativeVaultItemInput(
                title: "GitHub Personal Token", category: .apiKey,
                username: "dev@example.com", secretPayload: generatedPassword,
                tags: ["dev"], isFavorite: true
            ))
            _ = try engine.saveItem(input: NativeVaultItemInput(
                title: "Console de Produção", category: .login,
                username: "admin@example.com", secretPayload: "senha123",
                tags: ["infra"]
            ))
            _ = try engine.saveItem(input: NativeVaultItemInput(
                title: "Chave de recuperação", category: .note,
                secretPayload: "Guardar fora do computador: 4F7A-91C2-DD03"
            ))
        }
        regeneratePassword()
        reload()
    }

    private func perform(_ block: () throws -> Void) {
        do {
            try block()
        } catch {
            errorMessage = describe(error)
        }
    }
}

// =============================================================================
// Tela principal
// =============================================================================

struct ContentView: View {
    @State private var vm = VaultViewModel.persistent()
    @State private var isAdding = false

    private var isFiltering: Bool {
        !vm.searchQuery.isEmpty || vm.categoryFilter != nil
    }

    var body: some View {
        NavigationStack {
            List {
                if let message = vm.errorMessage {
                    Section {
                        Label(message, systemImage: "exclamationmark.triangle.fill")
                            .foregroundStyle(.red)
                            .onTapGesture { vm.errorMessage = nil }
                    }
                }

                // Cofre vazio: o convite fica no topo; no fim da lista ele
                // ficava escondido sob a barra de busca flutuante.
                if vm.items.isEmpty && !isFiltering {
                    Section {
                        EmptyVaultView(
                            isFiltering: false,
                            onAdd: { isAdding = true },
                            onLoadSamples: { vm.loadSampleItems() }
                        )
                    }
                }

                if let stats = vm.stats {
                    Section("Saúde do Cofre") {
                        HealthSummary(stats: stats)
                    }
                }

                Section("Gerador de Senhas (Rust Core)") {
                    PasswordGenerator(vm: vm)
                }

                Section {
                    Picker("Categoria", selection: $vm.categoryFilter) {
                        Text("Todas").tag(NativeCategory?.none)
                        ForEach(NativeCategory.allCases) { category in
                            Label(category.displayName, systemImage: category.systemImage)
                                .tag(Optional(category))
                        }
                    }

                    if vm.items.isEmpty {
                        if isFiltering {
                            EmptyVaultView(isFiltering: true, onAdd: {}, onLoadSamples: {})
                        } else {
                            Text("Nenhuma credencial salva ainda.")
                                .foregroundStyle(.secondary)
                        }
                    } else {
                        ForEach(vm.items) { item in
                            NavigationLink(value: item.id) {
                                VaultRow(item: item)
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    vm.delete(id: item.id)
                                } label: {
                                    Label("Excluir", systemImage: "trash")
                                }
                            }
                            .swipeActions(edge: .leading) {
                                Button {
                                    vm.toggleFavorite(id: item.id)
                                } label: {
                                    Label("Favorito", systemImage: item.isFavorite ? "star.slash" : "star")
                                }
                                .tint(.yellow)
                            }
                        }
                    }
                } header: {
                    Text("Credenciais (SQLite)")
                }
            }
            .navigationTitle("🔐 Secure Vault")
            .navigationDestination(for: String.self) { id in
                ItemDetailView(vm: vm, itemId: id)
            }
            .searchable(text: $vm.searchQuery, prompt: "Buscar por título, usuário ou tag")
            .onChange(of: vm.searchQuery) { vm.reload() }
            .onChange(of: vm.categoryFilter) { vm.reload() }
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        isAdding = true
                    } label: {
                        Label("Nova credencial", systemImage: "plus")
                    }
                    .accessibilityIdentifier("add-item")
                }
            }
            .sheet(isPresented: $isAdding) {
                ItemEditorView(vm: vm, existing: nil)
            }
        }
    }
}

struct HealthSummary: View {
    let stats: NativeVaultStats

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Pontuação")
                Spacer()
                Text("\(stats.healthScore)%")
                    .font(.title2.bold())
                    .foregroundStyle(strengthColor(stats.healthScore))
                    .accessibilityIdentifier("health-score")
            }
            ProgressView(value: Double(stats.healthScore), total: 100)
                .tint(strengthColor(stats.healthScore))
            HStack {
                StatBadge(value: stats.totalItems, label: "Itens")
                StatBadge(value: stats.favoritesCount, label: "Favoritos")
                StatBadge(value: stats.weakPasswordsCount, label: "Fracas")
            }
        }
        .padding(.vertical, 4)
    }
}

struct StatBadge: View {
    let value: UInt32
    let label: String

    var body: some View {
        VStack {
            Text("\(value)").font(.headline)
            Text(label).font(.caption).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
    }
}

struct PasswordGenerator: View {
    @Bindable var vm: VaultViewModel
    @State private var copied = false

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(vm.generatedPassword)
                .font(.system(.body, design: .monospaced).bold())
                .foregroundStyle(.blue)
                .textSelection(.enabled)
                .accessibilityIdentifier("generated-password")
            if let evaluation = vm.generatedEvaluation {
                StrengthMeter(evaluation: evaluation)
            }
            HStack {
                Text("Tamanho: \(Int(vm.passwordLength))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Slider(value: $vm.passwordLength, in: 8...64, step: 1)
                    .onChange(of: vm.passwordLength) { vm.regeneratePassword() }
            }
            HStack {
                Button {
                    vm.regeneratePassword()
                    copied = false
                } label: {
                    Label("Gerar nova", systemImage: "arrow.clockwise")
                }
                .accessibilityIdentifier("regenerate-password")
                Spacer()
                Button {
                    UIPasteboard.general.string = vm.generatedPassword
                    copied = true
                } label: {
                    Label(copied ? "Copiada" : "Copiar", systemImage: copied ? "checkmark" : "doc.on.doc")
                }
            }
            .buttonStyle(.borderless)
        }
    }
}

struct StrengthMeter: View {
    let evaluation: NativePasswordEvaluation

    var body: some View {
        HStack {
            ProgressView(value: Double(evaluation.score), total: 100)
                .tint(strengthColor(evaluation.score))
            Text("\(evaluation.label) (\(evaluation.score))")
                .font(.caption.bold())
                .foregroundStyle(strengthColor(evaluation.score))
        }
    }
}

struct VaultRow: View {
    let item: NativeVaultItem

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: item.category.systemImage)
                .foregroundStyle(.tint)
                .frame(width: 28)
            VStack(alignment: .leading, spacing: 3) {
                Text(item.title).font(.headline)
                Text([item.category.displayName, item.username].compactMap { $0 }.joined(separator: " • "))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if item.isFavorite {
                Image(systemName: "star.fill").foregroundStyle(.yellow)
            }
        }
    }
}

struct EmptyVaultView: View {
    let isFiltering: Bool
    let onAdd: () -> Void
    let onLoadSamples: () -> Void

    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: isFiltering ? "magnifyingglass" : "lock.shield")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text(isFiltering ? "Nenhuma credencial encontrada" : "Seu cofre está vazio")
                .font(.headline)
            if !isFiltering {
                Button("Adicionar credencial", action: onAdd)
                    .buttonStyle(.borderedProminent)
                Button("Carregar exemplos", action: onLoadSamples)
                    .buttonStyle(.borderless)
                    .accessibilityIdentifier("load-samples")
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical)
    }
}

// =============================================================================
// Detalhe
// =============================================================================

struct ItemDetailView: View {
    let vm: VaultViewModel
    let itemId: String
    @State private var item: NativeVaultItem?
    @State private var revealSecret = false
    @State private var isEditing = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        Group {
            if let item {
                List {
                    Section {
                        LabeledContent("Categoria", value: item.category.displayName)
                        if let username = item.username {
                            LabeledContent("Usuário", value: username)
                                .textSelection(.enabled)
                        }
                        if !item.tags.isEmpty {
                            LabeledContent("Tags", value: item.tags.joined(separator: ", "))
                        }
                    }
                    Section("Segredo") {
                        Text(revealSecret ? item.secretPayload : String(repeating: "•", count: 12))
                            .font(.system(.body, design: .monospaced))
                            .textSelection(.enabled)
                        StrengthMeter(evaluation: vm.evaluate(item.secretPayload))
                        HStack {
                            Button(revealSecret ? "Ocultar" : "Revelar") { revealSecret.toggle() }
                            Spacer()
                            Button("Copiar") { UIPasteboard.general.string = item.secretPayload }
                        }
                        .buttonStyle(.borderless)
                    }
                    Section {
                        LabeledContent(
                            "Atualizado",
                            value: Date(timeIntervalSince1970: TimeInterval(item.updatedAt))
                                .formatted(date: .abbreviated, time: .shortened)
                        )
                        Button(role: .destructive) {
                            vm.delete(id: item.id)
                            dismiss()
                        } label: {
                            Label("Excluir credencial", systemImage: "trash")
                        }
                    }
                }
                .navigationTitle(item.title)
                .toolbar {
                    ToolbarItemGroup(placement: .primaryAction) {
                        Button {
                            vm.toggleFavorite(id: item.id)
                            refresh()
                        } label: {
                            Image(systemName: item.isFavorite ? "star.fill" : "star")
                        }
                        Button("Editar") { isEditing = true }
                    }
                }
                .sheet(isPresented: $isEditing, onDismiss: refresh) {
                    ItemEditorView(vm: vm, existing: item)
                }
            } else {
                ContentUnavailableView("Item não encontrado", systemImage: "questionmark.folder")
            }
        }
        .onAppear(perform: refresh)
    }

    private func refresh() {
        item = vm.item(id: itemId)
    }
}

// =============================================================================
// Criação / edição
// =============================================================================

struct ItemEditorView: View {
    let vm: VaultViewModel
    let existing: NativeVaultItem?

    @State private var title = ""
    @State private var category: NativeCategory = .login
    @State private var username = ""
    @State private var secret = ""
    @State private var tags = ""
    @State private var isFavorite = false
    @State private var revealSecret = false
    @State private var errorMessage: String?
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Título", text: $title)
                        .accessibilityIdentifier("item-title")
                    Picker("Categoria", selection: $category) {
                        ForEach(NativeCategory.allCases) { category in
                            Text(category.displayName).tag(category)
                        }
                    }
                    TextField("Usuário / e-mail", text: $username)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.emailAddress)
                }
                Section("Segredo") {
                    HStack {
                        Group {
                            if revealSecret {
                                TextField("Senha ou conteúdo", text: $secret)
                            } else {
                                SecureField("Senha ou conteúdo", text: $secret)
                            }
                        }
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .font(.system(.body, design: .monospaced))
                        Button {
                            revealSecret.toggle()
                        } label: {
                            Image(systemName: revealSecret ? "eye.slash" : "eye")
                        }
                        .buttonStyle(.borderless)
                    }
                    if !secret.isEmpty {
                        StrengthMeter(evaluation: vm.evaluate(secret))
                    }
                    Button {
                        vm.regeneratePassword()
                        secret = vm.generatedPassword
                        revealSecret = true
                    } label: {
                        Label("Gerar senha forte (Rust)", systemImage: "wand.and.stars")
                    }
                }
                Section {
                    TextField("Tags separadas por vírgula", text: $tags)
                        .textInputAutocapitalization(.never)
                    Toggle("Favorito", isOn: $isFavorite)
                }
                if let errorMessage {
                    Section {
                        Text(errorMessage).foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle(existing == nil ? "Nova credencial" : "Editar credencial")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancelar") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Salvar", action: save)
                        .accessibilityIdentifier("save-item")
                }
            }
            .onAppear(perform: loadExisting)
        }
    }

    private func loadExisting() {
        guard let existing else { return }
        title = existing.title
        category = existing.category
        username = existing.username ?? ""
        secret = existing.secretPayload
        tags = existing.tags.joined(separator: ", ")
        isFavorite = existing.isFavorite
    }

    private func save() {
        let parsedTags = tags
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
        do {
            try vm.save(
                id: existing?.id,
                title: title,
                category: category,
                username: username,
                secret: secret,
                tags: parsedTags,
                isFavorite: isFavorite
            )
            dismiss()
        } catch {
            errorMessage = describe(error)
        }
    }
}

#Preview {
    ContentView()
}
