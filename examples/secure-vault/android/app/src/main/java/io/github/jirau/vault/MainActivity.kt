package io.github.jirau.vault

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import io.github.jirau.vault.uniffi.NativeCategory
import io.github.jirau.vault.uniffi.NativeVaultException
import io.github.jirau.vault.uniffi.NativeVaultItem
import io.github.jirau.vault.uniffi.NativeVaultStats
import io.github.jirau.vault.ui.theme.JirauTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            JirauTheme {
                Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    val context = LocalContext.current.applicationContext
                    VaultScreen(viewModel { VaultViewModel.persistent(context) })
                }
            }
        }
    }
}

fun strengthColor(score: UInt): Color = when {
    score >= 70u -> Color(0xFF2E7D32)
    score >= 40u -> Color(0xFFEF6C00)
    else -> Color(0xFFC62828)
}

fun copyToClipboard(context: Context, label: String, text: String) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
}

// =============================================================================
// Tela principal
// =============================================================================

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VaultScreen(vm: VaultViewModel) {
    val state by vm.state.collectAsState()
    var editing by remember { mutableStateOf<EditorTarget?>(null) }
    var detailId by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("🔐 Secure Vault", fontWeight = FontWeight.Bold) },
                actions = {
                    TextButton(
                        onClick = { editing = EditorTarget.New },
                        modifier = Modifier.testTag("add-item")
                    ) { Text("＋ Nova") }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer
                )
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            state.errorMessage?.let { message ->
                item {
                    Card(
                        onClick = vm::dismissError,
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer)
                    ) {
                        Text("⚠️ $message", modifier = Modifier.padding(16.dp).testTag("error-message"))
                    }
                }
            }

            // Cofre vazio: o convite fica no topo, sempre visível.
            if (state.items.isEmpty() && !state.isFiltering) {
                item {
                    EmptyVaultCard(onAdd = { editing = EditorTarget.New }, onLoadSamples = vm::loadSampleItems)
                }
            }

            state.stats?.let { stats -> item { HealthSummary(stats) } }

            item { PasswordGenerator(state, vm) }

            item {
                OutlinedTextField(
                    value = state.searchQuery,
                    onValueChange = vm::setSearchQuery,
                    label = { Text("Buscar por título, usuário ou tag") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("search")
                )
            }

            item {
                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = state.categoryFilter == null,
                        onClick = { vm.setCategoryFilter(null) },
                        label = { Text("Todas") }
                    )
                    NativeCategory.entries.forEach { category ->
                        FilterChip(
                            selected = state.categoryFilter == category,
                            onClick = { vm.setCategoryFilter(category) },
                            label = { Text("${category.emoji} ${category.displayName}") }
                        )
                    }
                }
            }

            item {
                Text("Credenciais (SQLite cifrado)", style = MaterialTheme.typography.titleMedium)
            }

            if (state.items.isEmpty()) {
                item {
                    Text(
                        if (state.isFiltering) "Nenhuma credencial encontrada." else "Nenhuma credencial salva ainda.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            items(state.items, key = { it.id }) { item ->
                VaultRow(
                    item = item,
                    onOpen = { detailId = item.id },
                    onToggleFavorite = { vm.toggleFavorite(item.id) }
                )
            }
        }
    }

    detailId?.let { id ->
        val item = vm.item(id)
        if (item == null) {
            detailId = null
        } else {
            ItemDetailDialog(
                item = item,
                vm = vm,
                onEdit = {
                    detailId = null
                    editing = EditorTarget.Existing(item)
                },
                onDelete = {
                    vm.delete(item.id)
                    detailId = null
                },
                onDismiss = { detailId = null }
            )
        }
    }

    editing?.let { target ->
        ItemEditorDialog(target = target, vm = vm, onDismiss = { editing = null })
    }
}

sealed interface EditorTarget {
    data object New : EditorTarget
    data class Existing(val item: NativeVaultItem) : EditorTarget
}

@Composable
fun EmptyVaultCard(onAdd: () -> Unit, onLoadSamples: () -> Unit) {
    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text("🛡️", style = MaterialTheme.typography.displaySmall)
            Text("Seu cofre está vazio", style = MaterialTheme.typography.titleMedium)
            Button(onClick = onAdd) { Text("Adicionar credencial") }
            TextButton(onClick = onLoadSamples, modifier = Modifier.testTag("load-samples")) {
                Text("Carregar exemplos")
            }
        }
    }
}

@Composable
fun HealthSummary(stats: NativeVaultStats) {
    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Saúde do Cofre", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                Text(
                    "${stats.healthScore}%",
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.Bold,
                    color = strengthColor(stats.healthScore),
                    modifier = Modifier.testTag("health-score")
                )
            }
            LinearProgressIndicator(
                progress = { stats.healthScore.toFloat() / 100f },
                color = strengthColor(stats.healthScore),
                modifier = Modifier.fillMaxWidth()
            )
            Row(horizontalArrangement = Arrangement.SpaceEvenly, modifier = Modifier.fillMaxWidth()) {
                StatBadge("${stats.totalItems}", "Itens")
                StatBadge("${stats.favoritesCount}", "Favoritos")
                StatBadge("${stats.weakPasswordsCount}", "Fracas")
            }
        }
    }
}

@Composable
fun StatBadge(value: String, label: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(value, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
fun PasswordGenerator(state: VaultUiState, vm: VaultViewModel) {
    val context = LocalContext.current
    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Gerador de Senhas (Rust Core)", style = MaterialTheme.typography.titleMedium)
            Text(
                state.generatedPassword,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.testTag("generated-password")
            )
            state.generatedEvaluation?.let { evaluation ->
                Text(
                    "${evaluation.label} (${evaluation.score})",
                    color = strengthColor(evaluation.score),
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.testTag("generated-strength")
                )
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Tamanho: ${state.passwordLength}", modifier = Modifier.width(110.dp))
                Slider(
                    value = state.passwordLength.toFloat(),
                    onValueChange = { vm.setPasswordLength(it.toInt()) },
                    valueRange = 8f..64f,
                    modifier = Modifier.weight(1f)
                )
            }
            Row {
                TextButton(onClick = vm::regeneratePassword, modifier = Modifier.testTag("regenerate-password")) {
                    Text("↻ Gerar nova")
                }
                Spacer(Modifier.weight(1f))
                TextButton(onClick = { copyToClipboard(context, "Senha", state.generatedPassword) }) {
                    Text("Copiar")
                }
            }
        }
    }
}

@Composable
fun VaultRow(item: NativeVaultItem, onOpen: () -> Unit, onToggleFavorite: () -> Unit) {
    Card(modifier = Modifier.fillMaxWidth().clickable(onClick = onOpen)) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(14.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(item.category.emoji, style = MaterialTheme.typography.titleLarge)
            Column(modifier = Modifier.weight(1f).padding(horizontal = 12.dp)) {
                Text(item.title, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                Text(
                    listOfNotNull(item.category.displayName, item.username).joinToString(" • "),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            TextButton(onClick = onToggleFavorite) {
                Text(if (item.isFavorite) "★" else "☆", style = MaterialTheme.typography.titleLarge)
            }
        }
    }
}

// =============================================================================
// Detalhe
// =============================================================================

@Composable
fun ItemDetailDialog(
    item: NativeVaultItem,
    vm: VaultViewModel,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    var revealed by remember { mutableStateOf(false) }
    val evaluation = remember(item.secretPayload) { vm.evaluate(item.secretPayload) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("${item.category.emoji} ${item.title}") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Categoria: ${item.category.displayName}")
                item.username?.let { Text("Usuário: $it") }
                if (item.tags.isNotEmpty()) Text("Tags: ${item.tags.joinToString(", ")}")
                Text(
                    if (revealed) item.secretPayload else "••••••••••••••••",
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.testTag("secret-value")
                )
                Text(
                    "Força: ${evaluation.label} (${evaluation.score})",
                    color = strengthColor(evaluation.score)
                )
                Row {
                    TextButton(onClick = { revealed = !revealed }, modifier = Modifier.testTag("reveal-secret")) {
                        Text(if (revealed) "Ocultar" else "Revelar")
                    }
                    TextButton(onClick = { copyToClipboard(context, item.title, item.secretPayload) }) {
                        Text("Copiar")
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onEdit) { Text("Editar") } },
        dismissButton = {
            Row {
                TextButton(onClick = onDelete) { Text("Excluir", color = MaterialTheme.colorScheme.error) }
                TextButton(onClick = onDismiss) { Text("Fechar") }
            }
        }
    )
}

// =============================================================================
// Editor
// =============================================================================

@Composable
fun ItemEditorDialog(target: EditorTarget, vm: VaultViewModel, onDismiss: () -> Unit) {
    val existing = (target as? EditorTarget.Existing)?.item
    var title by remember { mutableStateOf(existing?.title ?: "") }
    var category by remember { mutableStateOf(existing?.category ?: NativeCategory.LOGIN) }
    var username by remember { mutableStateOf(existing?.username ?: "") }
    var secret by remember { mutableStateOf(existing?.secretPayload ?: "") }
    var tags by remember { mutableStateOf(existing?.tags?.joinToString(", ") ?: "") }
    var isFavorite by remember { mutableStateOf(existing?.isFavorite ?: false) }
    var showSecret by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (existing == null) "Nova credencial" else "Editar credencial") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Título") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("item-title")
                )
                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    NativeCategory.entries.forEach { option ->
                        FilterChip(
                            selected = category == option,
                            onClick = { category = option },
                            label = { Text(option.displayName) }
                        )
                    }
                }
                OutlinedTextField(
                    value = username,
                    onValueChange = { username = it },
                    label = { Text("Usuário (opcional)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = secret,
                    onValueChange = { secret = it },
                    label = { Text("Segredo") },
                    singleLine = true,
                    visualTransformation = if (showSecret) VisualTransformation.None else PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    modifier = Modifier.fillMaxWidth().testTag("item-secret")
                )
                if (secret.isNotEmpty()) {
                    val evaluation = vm.evaluate(secret)
                    Text("Força: ${evaluation.label} (${evaluation.score})", color = strengthColor(evaluation.score))
                }
                Row {
                    TextButton(
                        onClick = {
                            secret = vm.newPassword()
                            showSecret = true
                        },
                        modifier = Modifier.testTag("generate-secret")
                    ) { Text("Gerar senha forte (Rust)") }
                    TextButton(onClick = { showSecret = !showSecret }) {
                        Text(if (showSecret) "Ocultar" else "Mostrar")
                    }
                }
                OutlinedTextField(
                    value = tags,
                    onValueChange = { tags = it },
                    label = { Text("Tags (separadas por vírgula)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Favorito", modifier = Modifier.weight(1f))
                    Switch(checked = isFavorite, onCheckedChange = { isFavorite = it })
                }
                error?.let {
                    Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.testTag("editor-error"))
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    try {
                        vm.save(
                            id = existing?.id,
                            title = title,
                            category = category,
                            username = username,
                            secret = secret,
                            tags = tags.split(",").map { it.trim() }.filter { it.isNotEmpty() },
                            isFavorite = isFavorite,
                        )
                        onDismiss()
                    } catch (e: NativeVaultException) {
                        // Validação feita no Rust (título/segredo vazios etc.).
                        error = describe(e)
                    }
                },
                modifier = Modifier.testTag("save-item")
            ) { Text("Salvar") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancelar") } }
    )
}
