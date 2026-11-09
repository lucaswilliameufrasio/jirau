package io.github.jirau.vault

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

data class VaultItemUi(
    val id: String,
    val title: String,
    val category: String,
    val username: String,
    val secretPayload: String,
    val isFavorite: Boolean
)

class VaultViewModel : ViewModel() {
    private val _items = MutableStateFlow<List<VaultItemUi>>(
        listOf(
            VaultItemUi("1", "GitHub Personal Token", "API Key", "lucas@dev.io", "ghp_secureKeyExample999", true),
            VaultItemUi("2", "AWS Production Console", "Login", "admin@corp.cloud", "P@ssw0rdProd#2026", true),
            VaultItemUi("3", "Chave Mestra de Criptografia", "Nota", "", "AES256-GCM-PayloadKey-Local", false)
        )
    )
    val items: StateFlow<List<VaultItemUi>> = _items.asStateFlow()

    private val _generatedPassword = MutableStateFlow("S3cur3#P@ss-2026")
    val generatedPassword: StateFlow<String> = _generatedPassword.asStateFlow()

    private val _healthScore = MutableStateFlow(92)
    val healthScore: StateFlow<Int> = _healthScore.asStateFlow()

    fun generateNewPassword() {
        val chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%&*"
        val pwd = (1..18).map { chars.random() }.joinToString("")
        _generatedPassword.value = pwd
    }

    fun toggleFavorite(id: String) {
        _items.value = _items.value.map {
            if (it.id == id) it.copy(isFavorite = !it.isFavorite) else it
        }
    }
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    VaultScreen()
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VaultScreen(vm: VaultViewModel = viewModel()) {
    val items by vm.items.collectAsState()
    val generatedPassword by vm.generatedPassword.collectAsState()
    val healthScore by vm.healthScore.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("🔐 Secure Vault Vault", style = MaterialTheme.typography.titleLarge)
                        Text("Saúde do Cofre: $healthScore% Seguro", style = MaterialTheme.typography.labelSmall)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer
                )
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Card do Gerador de Senha
            ElevatedCard(
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Text("Gerador Criptográfico (Rust)", style = MaterialTheme.typography.titleMedium)
                    Text(generatedPassword, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.primary)
                    Button(
                        onClick = { vm.generateNewPassword() },
                        modifier = Modifier.align(Alignment.End)
                    ) {
                        Text("⚡ Gerar Senha")
                    }
                }
            }

            Text("Credenciais Salvas (SQLite):", style = MaterialTheme.typography.titleMedium)

            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(items) { item ->
                    Card(
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(14.dp),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(item.title, style = MaterialTheme.typography.bodyLarge)
                                Text("${item.category} • ${item.username}", style = MaterialTheme.typography.bodySmall)
                                Text("••••••••••••••••", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                            }
                            IconButton(onClick = { vm.toggleFavorite(item.id) }) {
                                Text(if (item.isFavorite) "★" else "☆")
                            }
                        }
                    }
                }
            }
        }
    }
}
