package io.github.jirau.vault

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.jirau.vault.uniffi.NativeCategory
import io.github.jirau.vault.uniffi.NativeVaultEngine
import io.github.jirau.vault.uniffi.NativeVaultException
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** ViewModel e UI contra o núcleo Rust real (SQLite em memória via UniFFI). */
@RunWith(AndroidJUnit4::class)
class NativeLifecycleTest {
    @get:Rule
    val compose = createComposeRule()

    private fun makeViewModel() = VaultViewModel(NativeVaultEngine.inMemory())

    @Test
    fun emptyVaultStartsWithPerfectHealth() {
        val state = makeViewModel().state.value
        assertTrue(state.items.isEmpty())
        assertEquals(0u, state.stats?.totalItems)
        assertEquals(100u, state.stats?.healthScore)
    }

    @Test
    fun saveSearchFavoriteAndDeleteRoundTripThroughRust() {
        val vm = makeViewModel()
        val saved = vm.save(null, "GitHub", NativeCategory.API_KEY, "dev@example.com", "ghp_Abc123!xyz7890Qw", listOf("dev"), false)
        vm.save(null, "Banco", NativeCategory.LOGIN, "", "senha123", emptyList(), false)
        assertEquals(2, vm.state.value.items.size)
        assertEquals(1u, vm.state.value.stats?.weakPasswordsCount)
        val banco = vm.state.value.items.first { it.title == "Banco" }
        assertNull("usuário vazio deve ser persistido como nulo", vm.item(banco.id)?.username)

        vm.setSearchQuery("git")
        assertEquals(listOf("GitHub"), vm.state.value.items.map { it.title })

        vm.setSearchQuery("")
        vm.setCategoryFilter(NativeCategory.LOGIN)
        assertEquals(listOf("Banco"), vm.state.value.items.map { it.title })

        vm.setCategoryFilter(null)
        vm.toggleFavorite(saved.id)
        assertEquals(true, vm.item(saved.id)?.isFavorite)
        assertEquals("favoritos vêm primeiro", saved.id, vm.state.value.items.first().id)

        vm.delete(saved.id)
        assertNull(vm.item(saved.id))
        assertEquals(1u, vm.state.value.stats?.totalItems)
        assertNull(vm.state.value.errorMessage)
    }

    @Test
    fun rustRejectsInvalidInput() {
        val vm = makeViewModel()
        try {
            vm.save(null, "   ", NativeCategory.LOGIN, "", "x", emptyList(), false)
            fail("o Rust deveria recusar título vazio")
        } catch (e: NativeVaultException.InvalidInput) {
            assertEquals("Título e segredo são obrigatórios.", describe(e))
        }
        assertTrue(vm.state.value.items.isEmpty())
    }

    @Test
    fun generatedPasswordComesFromRustAndIsStrong() {
        val vm = makeViewModel()
        vm.setPasswordLength(24)
        val first = vm.state.value.generatedPassword
        assertEquals(24, first.length)
        assertEquals(true, vm.state.value.generatedEvaluation?.isSecure)

        vm.regeneratePassword()
        assertNotEquals(first, vm.state.value.generatedPassword)
    }

    @Test
    fun keystoreKeyIsStableAndOpensTheSameVault() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val key = VaultKeyStore.loadOrCreate(context)
        assertEquals(32, key.size)
        assertArrayEquals(key, VaultKeyStore.loadOrCreate(context))

        val path = context.cacheDir.resolve("keystore-test.sqlite3").also { it.delete() }.absolutePath
        val id = NativeVaultEngine.open(path, key).use { engine ->
            VaultViewModel(engine).save(null, "Banco", NativeCategory.LOGIN, "", "S3gredo!Forte#2026", emptyList(), false).id
        }
        NativeVaultEngine.open(path, VaultKeyStore.loadOrCreate(context)).use { engine ->
            assertEquals("S3gredo!Forte#2026", engine.getItem(id).secretPayload)
        }
    }

    @Test
    fun uiCreatesCredentialAndShowsRustValidation() {
        val vm = makeViewModel()
        compose.setContent { VaultScreen(vm) }

        // Título vazio: a validação vem do Rust e aparece no editor.
        compose.onNodeWithTag("add-item").performClick()
        compose.onNodeWithTag("save-item").performClick()
        compose.onNodeWithTag("editor-error").assertIsDisplayed()

        compose.onNodeWithTag("item-title").performTextInput("Conta Banco Digital")
        compose.onNodeWithTag("generate-secret").performClick()
        compose.onNodeWithTag("save-item").performClick()

        compose.onNodeWithText("Conta Banco Digital").assertIsDisplayed()
        assertEquals(1, vm.state.value.items.size)
        assertTrue(vm.evaluate(vm.state.value.items.first().secretPayload).isSecure)
    }
}
