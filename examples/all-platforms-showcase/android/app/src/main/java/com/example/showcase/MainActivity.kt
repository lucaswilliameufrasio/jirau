package com.example.showcase

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.showcase.ui.theme.JirauTheme
import com.example.showcase.uniffi.AppEngine
import com.example.showcase.uniffi.NativeException
import com.example.showcase.uniffi.nativeGreet
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Estado e regras vivem no núcleo Rust; a ViewModel só repassa as chamadas (UniFFI). */
class MainViewModel(private val engine: AppEngine = AppEngine()) : ViewModel() {
    private val _counter = MutableStateFlow(engine.getState().counter)
    val counter: StateFlow<Int> = _counter.asStateFlow()

    private val _status = MutableStateFlow(nativeGreet("Android"))
    val status: StateFlow<String> = _status.asStateFlow()

    fun increment() {
        _counter.value = engine.increment().counter
        _status.value = "Incrementado pelo núcleo Rust"
    }

    fun decrement() {
        try {
            _counter.value = engine.decrement().counter
            _status.value = "Decrementado pelo núcleo Rust"
        } catch (e: NativeException.NegativeValue) {
            _status.value = "Rust recusou: o contador não pode ficar negativo"
        }
    }

    override fun onCleared() {
        engine.close()
    }
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            JirauTheme {
                Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    AppScreen()
                }
            }
        }
    }
}

@Composable
fun AppScreen(vm: MainViewModel = viewModel()) {
    val counter by vm.counter.collectAsState()
    val status by vm.status.collectAsState()

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text("🪵 all-platforms-showcase", style = MaterialTheme.typography.headlineMedium)
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            "UI nativa em Jetpack Compose + núcleo em Rust",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.secondary
        )
        Spacer(modifier = Modifier.height(32.dp))

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text("Contador (Rust Core)", style = MaterialTheme.typography.labelLarge)
                Text(
                    "$counter",
                    style = MaterialTheme.typography.displayLarge,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("counter")
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(status, style = MaterialTheme.typography.bodySmall, modifier = Modifier.testTag("status"))
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly) {
            Button(
                onClick = vm::decrement,
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                modifier = Modifier.testTag("decrement")
            ) { Text("− Menos") }
            Button(onClick = vm::increment, modifier = Modifier.testTag("increment")) { Text("+ Mais") }
        }
    }
}
