package io.github.jirau.vault

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.lifecycle.ViewModel
import io.github.jirau.vault.uniffi.NativeCategory
import io.github.jirau.vault.uniffi.NativePasswordEvaluation
import io.github.jirau.vault.uniffi.NativePasswordOptions
import io.github.jirau.vault.uniffi.NativeVaultEngine
import io.github.jirau.vault.uniffi.NativeVaultException
import io.github.jirau.vault.uniffi.NativeVaultItem
import io.github.jirau.vault.uniffi.NativeVaultItemInput
import io.github.jirau.vault.uniffi.NativeVaultStats
import io.github.jirau.vault.uniffi.evaluatePassword
import io.github.jirau.vault.uniffi.generateSecurePassword
import io.github.jirau.vault.uniffi.generateVaultKey
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

// =============================================================================
// Integração com o núcleo Rust (UniFFI)
// `NativeVaultEngine`, `generateSecurePassword`, `evaluatePassword` e
// `generateVaultKey` vêm de app/build/generated/uniffi
// (scripts/build-android-ffi.sh, executado pelo Gradle antes do build).
// =============================================================================

val NativeCategory.displayName: String
    get() = when (this) {
        NativeCategory.LOGIN -> "Login"
        NativeCategory.NOTE -> "Nota"
        NativeCategory.API_KEY -> "Chave de API"
        NativeCategory.CREDIT_CARD -> "Cartão"
    }

val NativeCategory.emoji: String
    get() = when (this) {
        NativeCategory.LOGIN -> "🔑"
        NativeCategory.NOTE -> "📝"
        NativeCategory.API_KEY -> "⚡"
        NativeCategory.CREDIT_CARD -> "💳"
    }

fun describe(error: Throwable): String = when (error) {
    is NativeVaultException.InvalidInput -> "Título e segredo são obrigatórios."
    is NativeVaultException.NotFound -> "Item não encontrado (${error.id})."
    is NativeVaultException.Database -> "Erro no banco de dados: ${error.detail}"
    is NativeVaultException.InvalidKey -> "Chave do cofre inválida."
    is NativeVaultException.WrongKeyOrCorrupted ->
        "A chave do Keystore não abre este cofre (chave trocada ou dados corrompidos)."
    else -> error.message ?: error.toString()
}

// =============================================================================
// Chave do cofre no Android Keystore
// O Rust cifra cada segredo com AES-256-GCM-SIV (aws-lc-rs) usando uma chave de 32 bytes
// gerada pelo próprio núcleo. Essa chave só é gravada embrulhada (AES-GCM) por
// uma chave do Android Keystore, que nunca sai do hardware/TEE do aparelho.
// =============================================================================

object VaultKeyStore {
    private const val KEYSTORE = "AndroidKeyStore"
    private const val WRAPPING_ALIAS = "vault-master-key-wrapper"
    private const val PREFS = "vault_key"
    private const val PREF_WRAPPED_KEY = "wrapped_vault_key"
    private const val GCM_IV_BYTES = 12
    private const val GCM_TAG_BITS = 128

    /** Lê a chave do cofre; na primeira execução gera uma nova e a guarda. */
    fun loadOrCreate(context: Context): ByteArray {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        prefs.getString(PREF_WRAPPED_KEY, null)?.let { stored ->
            return unwrap(Base64.decode(stored, Base64.NO_WRAP))
        }
        val key = generateVaultKey()
        val wrapped = Base64.encodeToString(wrap(key), Base64.NO_WRAP)
        check(prefs.edit().putString(PREF_WRAPPED_KEY, wrapped).commit()) {
            "Não foi possível salvar a chave do cofre"
        }
        return key
    }

    private fun wrappingKey(): SecretKey {
        val keyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }
        (keyStore.getKey(WRAPPING_ALIAS, null) as? SecretKey)?.let { return it }

        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                WRAPPING_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        return generator.generateKey()
    }

    private fun wrap(plain: ByteArray): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, wrappingKey())
        return cipher.iv + cipher.doFinal(plain)
    }

    private fun unwrap(blob: ByteArray): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            wrappingKey(),
            GCMParameterSpec(GCM_TAG_BITS, blob, 0, GCM_IV_BYTES)
        )
        return cipher.doFinal(blob, GCM_IV_BYTES, blob.size - GCM_IV_BYTES)
    }
}

// =============================================================================
// ViewModel: toda regra de negócio e persistência acontece no Rust.
// =============================================================================

data class VaultUiState(
    val items: List<NativeVaultItem> = emptyList(),
    val stats: NativeVaultStats? = null,
    val searchQuery: String = "",
    val categoryFilter: NativeCategory? = null,
    val passwordLength: Int = 20,
    val generatedPassword: String = "",
    val generatedEvaluation: NativePasswordEvaluation? = null,
    val errorMessage: String? = null,
) {
    val isFiltering: Boolean get() = searchQuery.isNotBlank() || categoryFilter != null
}

class VaultViewModel(private val engine: NativeVaultEngine) : ViewModel() {
    private val _state = MutableStateFlow(VaultUiState())
    val state: StateFlow<VaultUiState> = _state.asStateFlow()

    init {
        regeneratePassword()
        reload()
    }

    fun reload() = perform {
        val current = _state.value
        val query = current.searchQuery.trim().ifEmpty { null }
        val items = engine.listItems(current.categoryFilter, query)
        val stats = engine.getStats()
        _state.update { it.copy(items = items, stats = stats) }
    }

    fun setSearchQuery(query: String) {
        _state.update { it.copy(searchQuery = query) }
        reload()
    }

    fun setCategoryFilter(category: NativeCategory?) {
        _state.update { it.copy(categoryFilter = category) }
        reload()
    }

    fun item(id: String): NativeVaultItem? = runCatching { engine.getItem(id) }.getOrNull()

    /** Salva no Rust; lança [NativeVaultException] para a UI exibir a validação. */
    fun save(
        id: String?,
        title: String,
        category: NativeCategory,
        username: String,
        secret: String,
        tags: List<String>,
        isFavorite: Boolean,
    ): NativeVaultItem {
        val item = engine.saveItem(
            NativeVaultItemInput(
                id = id,
                title = title,
                category = category,
                username = username.trim().ifEmpty { null },
                secretPayload = secret,
                tags = tags,
                isFavorite = isFavorite,
            )
        )
        reload()
        return item
    }

    fun delete(id: String) {
        perform { engine.deleteItem(id) }
        reload()
    }

    fun toggleFavorite(id: String) {
        perform { engine.toggleFavorite(id) }
        reload()
    }

    fun setPasswordLength(length: Int) {
        _state.update { it.copy(passwordLength = length) }
        regeneratePassword()
    }

    fun regeneratePassword() {
        val password = newPassword(_state.value.passwordLength)
        _state.update {
            it.copy(generatedPassword = password, generatedEvaluation = evaluatePassword(password))
        }
    }

    fun newPassword(length: Int = 20): String = generateSecurePassword(
        NativePasswordOptions(
            length = length.toUInt(),
            includeUppercase = true,
            includeLowercase = true,
            includeNumbers = true,
            includeSymbols = true,
        )
    )

    fun evaluate(password: String): NativePasswordEvaluation = evaluatePassword(password)

    fun loadSampleItems() {
        perform {
            engine.saveItem(
                NativeVaultItemInput(
                    title = "GitHub Personal Token",
                    category = NativeCategory.API_KEY,
                    username = "dev@example.com",
                    secretPayload = _state.value.generatedPassword,
                    tags = listOf("dev"),
                    isFavorite = true,
                )
            )
            engine.saveItem(
                NativeVaultItemInput(
                    title = "Console de Produção",
                    category = NativeCategory.LOGIN,
                    username = "admin@example.com",
                    secretPayload = "senha123",
                    tags = listOf("infra"),
                )
            )
            engine.saveItem(
                NativeVaultItemInput(
                    title = "Chave de recuperação",
                    category = NativeCategory.NOTE,
                    secretPayload = "Guardar fora do computador: 4F7A-91C2-DD03",
                )
            )
        }
        reload()
    }

    fun dismissError() = _state.update { it.copy(errorMessage = null) }

    fun reportError(message: String) = _state.update { it.copy(errorMessage = message) }

    private inline fun perform(block: () -> Unit) {
        try {
            block()
        } catch (e: NativeVaultException) {
            _state.update { it.copy(errorMessage = describe(e)) }
        }
    }

    override fun onCleared() {
        engine.close()
    }

    companion object {
        /** Cofre em SQLite (fora de backup) com a chave no Android Keystore. */
        fun persistent(context: Context): VaultViewModel = try {
            val path = File(context.noBackupFilesDir, "vault.sqlite3").absolutePath
            val key = VaultKeyStore.loadOrCreate(context)
            VaultViewModel(NativeVaultEngine.open(path, key))
        } catch (e: Exception) {
            // Sem disco ou sem Keystore o app segue com um cofre temporário:
            // nada é gravado em disco sem criptografia.
            VaultViewModel(NativeVaultEngine.inMemory()).also {
                it.reportError("Cofre em disco indisponível (${describe(e)}). Usando cofre temporário.")
            }
        }
    }
}
