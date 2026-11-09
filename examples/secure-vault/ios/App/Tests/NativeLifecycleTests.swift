import XCTest
@testable import SecureVault

/// Testes do ViewModel contra o núcleo Rust real (SQLite em memória via UniFFI).
final class NativeLifecycleTests: XCTestCase {
    private func makeViewModel() throws -> VaultViewModel {
        VaultViewModel(engine: try NativeVaultEngine.inMemory())
    }

    func testEmptyVaultStartsWithPerfectHealth() throws {
        let vm = try makeViewModel()
        XCTAssertTrue(vm.items.isEmpty)
        XCTAssertEqual(vm.stats?.totalItems, 0)
        XCTAssertEqual(vm.stats?.healthScore, 100)
    }

    func testSaveSearchFavoriteAndDeleteRoundTripThroughRust() throws {
        let vm = try makeViewModel()
        let saved = try vm.save(
            id: nil, title: "GitHub", category: .apiKey, username: "dev@example.com",
            secret: "ghp_Abc123!xyz7890Qw", tags: ["dev"], isFavorite: false
        )
        try vm.save(
            id: nil, title: "Banco", category: .login, username: "",
            secret: "senha123", tags: [], isFavorite: false
        )
        XCTAssertEqual(vm.items.count, 2)
        XCTAssertEqual(vm.stats?.weakPasswordsCount, 1)
        XCTAssertNil(vm.item(id: vm.items.first { $0.title == "Banco" }!.id)?.username,
                     "usuário vazio deve ser persistido como nulo")

        vm.searchQuery = "git"
        vm.reload()
        XCTAssertEqual(vm.items.map(\.title), ["GitHub"])

        vm.searchQuery = ""
        vm.categoryFilter = .login
        vm.reload()
        XCTAssertEqual(vm.items.map(\.title), ["Banco"])

        vm.categoryFilter = nil
        vm.toggleFavorite(id: saved.id)
        XCTAssertEqual(vm.item(id: saved.id)?.isFavorite, true)
        XCTAssertEqual(vm.items.first?.id, saved.id, "favoritos vêm primeiro")

        vm.delete(id: saved.id)
        XCTAssertNil(vm.item(id: saved.id))
        XCTAssertEqual(vm.stats?.totalItems, 1)
        XCTAssertNil(vm.errorMessage)
    }

    func testEditingKeepsIdentity() throws {
        let vm = try makeViewModel()
        let saved = try vm.save(
            id: nil, title: "Original", category: .note, username: "",
            secret: "conteudo", tags: [], isFavorite: false
        )
        try vm.save(
            id: saved.id, title: "Renomeado", category: .note, username: "",
            secret: "conteudo", tags: ["a", "b"], isFavorite: true
        )
        XCTAssertEqual(vm.items.count, 1)
        XCTAssertEqual(vm.item(id: saved.id)?.title, "Renomeado")
        XCTAssertEqual(vm.item(id: saved.id)?.tags, ["a", "b"])
    }

    func testRustRejectsInvalidInput() throws {
        let vm = try makeViewModel()
        XCTAssertThrowsError(
            try vm.save(id: nil, title: "   ", category: .login, username: "",
                        secret: "x", tags: [], isFavorite: false)
        ) { error in
            guard case NativeVaultError.InvalidInput = error else {
                return XCTFail("erro inesperado: \(error)")
            }
        }
        XCTAssertTrue(vm.items.isEmpty)
    }

    func testKeychainKeyIsStableAndOpensTheSameVault() throws {
        // Falha com errSecMissingEntitlement (-34018) se o app não for assinado.
        let key = try VaultKeyStore.loadOrCreate()
        XCTAssertEqual(key.count, 32)
        XCTAssertEqual(try VaultKeyStore.loadOrCreate(), key)

        let path = FileManager.default.temporaryDirectory
            .appendingPathComponent("keychain-test-\(UUID().uuidString).sqlite3").path
        let id = try NativeVaultEngine.open(path: path, key: key).saveItem(input: NativeVaultItemInput(
            title: "Banco", category: .login, secretPayload: "S3gredo!Forte#2026"
        )).id
        let reopened = try NativeVaultEngine.open(path: path, key: try VaultKeyStore.loadOrCreate())
        XCTAssertEqual(try reopened.getItem(id: id).secretPayload, "S3gredo!Forte#2026")
    }

    func testGeneratedPasswordComesFromRustAndIsStrong() throws {
        let vm = try makeViewModel()
        vm.passwordLength = 24
        vm.regeneratePassword()
        let first = vm.generatedPassword
        XCTAssertEqual(first.count, 24)
        XCTAssertEqual(vm.generatedEvaluation?.isSecure, true)

        vm.regeneratePassword()
        XCTAssertNotEqual(vm.generatedPassword, first)
    }
}
