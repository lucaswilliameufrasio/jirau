use super::{TemplateContext, versions};

pub fn settings_gradle_kts(ctx: &TemplateContext) -> String {
    format!(
        r#"pluginManagement {{
    repositories {{
        google {{
            content {{
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }}
        }}
        mavenCentral()
        gradlePluginPortal()
    }}
}}
dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        google()
        mavenCentral()
    }}
}}

rootProject.name = "{slug}"
include(":app")
"#,
        slug = ctx.slug
    )
}

pub fn build_android_ffi_sh(ctx: &TemplateContext) -> String {
    r#"#!/usr/bin/env bash
# ==============================================================================
# Compila o núcleo Rust para Android (cargo-ndk → jniLibs) e gera os bindings
# Kotlin (UniFFI). Gerado via jirau — o Gradle chama este script no preBuild,
# então funciona também pelo Android Studio, sem a CLI.
#
# Uso: scripts/build-android-ffi.sh [debug|release] [ABI...]
#   ABIs padrão: arm64-v8a x86_64 (ou a variável ANDROID_ABIS)
# ==============================================================================
set -euo pipefail

PROFILE="${1:-debug}"
shift || true
CRATE="__SLUG__-ffi"
LIB="__RUST_NAME___ffi"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

case "$PROFILE" in
  debug) RELEASE_FLAG="" ;;
  release) RELEASE_FLAG="--release" ;;
  *)
    echo "Perfil inválido: $PROFILE (use 'debug' ou 'release')" >&2
    exit 2
    ;;
esac

if [ "$#" -gt 0 ]; then
  ABIS=("$@")
else
  read -r -a ABIS <<< "${ANDROID_ABIS:-arm64-v8a x86_64}"
fi

# Garante ~/.cargo/bin no PATH (o Gradle/Android Studio nem sempre herda o shell).
if [ -d "$HOME/.cargo/bin" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

# SDK/NDK: respeita as variáveis do usuário e cai para os caminhos padrão.
if [ -z "${ANDROID_HOME:-}" ]; then
  for d in "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk" "$HOME/Library/Android/sdk"; do
    if [ -n "$d" ] && [ -d "$d" ]; then
      export ANDROID_HOME="$d"
      break
    fi
  done
fi
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME/ndk" ]; then
  NDK_VERSION="$(ls "$ANDROID_HOME/ndk" | sort -V | tail -1)"
  if [ -n "$NDK_VERSION" ]; then
    export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
  fi
fi
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "error: Android NDK não encontrado. Instale com: sdkmanager 'ndk;28.2.13676358'" >&2
  exit 1
fi

if ! cargo ndk --version >/dev/null 2>&1; then
  echo "➜ Instalando cargo-ndk..."
  cargo install cargo-ndk --locked
fi

abi_to_target() {
  case "$1" in
    arm64-v8a) echo "aarch64-linux-android" ;;
    armeabi-v7a) echo "armv7-linux-androideabi" ;;
    x86_64) echo "x86_64-linux-android" ;;
    x86) echo "i686-linux-android" ;;
    *) echo "error: ABI desconhecida: $1" >&2; exit 2 ;;
  esac
}

NDK_ARGS=()
if command -v rustup >/dev/null 2>&1; then
  INSTALLED="$(rustup target list --installed)"
fi
for abi in "${ABIS[@]}"; do
  target="$(abi_to_target "$abi")"
  if [ -n "${INSTALLED:-}" ] && ! grep -qx "$target" <<< "$INSTALLED"; then
    echo "➜ Instalando target Rust $target..."
    rustup target add "$target"
  fi
  NDK_ARGS+=(-t "$abi")
done

# 1. Biblioteca dinâmica (.so) por ABI em android/app/src/main/jniLibs/<abi>/
JNI_DIR="android/app/src/main/jniLibs"
echo "➜ cargo ndk (${ABIS[*]}, $PROFILE)"
cargo ndk "${NDK_ARGS[@]}" --platform 26 -o "$JNI_DIR" build -p "$CRATE" --lib $RELEASE_FLAG

# 2. Bindings Kotlin: library mode lê os metadados UniFFI de um build de host
#    em debug (o perfil release usa strip, que remove esses metadados).
case "$(uname -s)" in
  Darwin) HOST_LIB="target/debug/lib${LIB}.dylib" ;;
  MINGW*|MSYS*|CYGWIN*) HOST_LIB="target/debug/${LIB}.dll" ;;
  *) HOST_LIB="target/debug/lib${LIB}.so" ;;
esac
echo "➜ Gerando bindings Kotlin via UniFFI"
cargo build -p "$CRATE" --lib
OUT="android/app/build/generated/uniffi"
rm -rf "$OUT"
mkdir -p "$OUT"
cargo run -q -p "$CRATE" --bin uniffi-bindgen -- generate --library "$HOST_LIB" --language kotlin --no-format --out-dir "$OUT"

echo "✅ Núcleo Rust pronto para Android: $JNI_DIR + $OUT"
"#
    .replace("__SLUG__", &ctx.slug)
    .replace("__RUST_NAME__", &ctx.rust_name)
}

pub fn gradle_properties() -> String {
    r#"org.gradle.jvmargs=-Xmx4g -XX:+UseParallelGC -Dfile.encoding=UTF-8
org.gradle.caching=true
org.gradle.configuration-cache=true
org.gradle.parallel=true
kotlin.daemon.jvmargs=-Xmx2g
"#
    .to_string()
}

/// Gradle Wrapper: scripts e jar oficiais (gerados por `gradle wrapper`) mais as
/// propriedades com a versão e o checksum da distribuição de `versions.rs`.
pub const GRADLEW: &str = include_str!("../../assets/gradle-wrapper/gradlew");
pub const GRADLEW_BAT: &str = include_str!("../../assets/gradle-wrapper/gradlew.bat");
pub const GRADLE_WRAPPER_JAR: &[u8] =
    include_bytes!("../../assets/gradle-wrapper/gradle-wrapper.jar");

pub fn gradle_wrapper_properties() -> String {
    format!(
        r#"distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-{gradle}-bin.zip
distributionSha256Sum={sha}
networkTimeout=10000
validateDistributionUrl=true
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
"#,
        gradle = versions::GRADLE,
        sha = versions::GRADLE_DIST_SHA256
    )
}

pub fn libs_versions_toml() -> String {
    format!(
        r#"# Versões conferidas nos registros oficiais em {verified_on} (jirau).
[versions]
agp = "{agp}"
kotlin = "{kotlin}"
coreKtx = "{core_ktx}"
lifecycle = "{lifecycle}"
activityCompose = "{activity_compose}"
composeBom = "{compose_bom}"
jna = "{jna}"
coroutines = "{coroutines}"
junit = "{junit}"
androidxTestExtJunit = "{test_ext_junit}"
androidxTestRunner = "{test_runner}"

[libraries]
androidx-core-ktx = {{ group = "androidx.core", name = "core-ktx", version.ref = "coreKtx" }}
androidx-lifecycle-runtime-ktx = {{ group = "androidx.lifecycle", name = "lifecycle-runtime-ktx", version.ref = "lifecycle" }}
androidx-lifecycle-viewmodel-compose = {{ group = "androidx.lifecycle", name = "lifecycle-viewmodel-compose", version.ref = "lifecycle" }}
androidx-activity-compose = {{ group = "androidx.activity", name = "activity-compose", version.ref = "activityCompose" }}
androidx-compose-bom = {{ group = "androidx.compose", name = "compose-bom", version.ref = "composeBom" }}
androidx-ui = {{ group = "androidx.compose.ui", name = "ui" }}
androidx-ui-graphics = {{ group = "androidx.compose.ui", name = "ui-graphics" }}
androidx-ui-tooling = {{ group = "androidx.compose.ui", name = "ui-tooling" }}
androidx-ui-tooling-preview = {{ group = "androidx.compose.ui", name = "ui-tooling-preview" }}
androidx-material3 = {{ group = "androidx.compose.material3", name = "material3" }}
kotlinx-coroutines-core = {{ group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-core", version.ref = "coroutines" }}
junit = {{ group = "junit", name = "junit", version.ref = "junit" }}
androidx-test-ext-junit = {{ group = "androidx.test.ext", name = "junit", version.ref = "androidxTestExtJunit" }}
androidx-test-runner = {{ group = "androidx.test", name = "runner", version.ref = "androidxTestRunner" }}
androidx-ui-test-junit4 = {{ group = "androidx.compose.ui", name = "ui-test-junit4" }}
androidx-ui-test-manifest = {{ group = "androidx.compose.ui", name = "ui-test-manifest" }}

[plugins]
android-application = {{ id = "com.android.application", version.ref = "agp" }}
kotlin-android = {{ id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }}
kotlin-compose = {{ id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }}
"#,
        verified_on = versions::VERIFIED_ON,
        agp = versions::AGP,
        kotlin = versions::KOTLIN,
        core_ktx = versions::ANDROIDX_CORE_KTX,
        lifecycle = versions::ANDROIDX_LIFECYCLE,
        activity_compose = versions::ANDROIDX_ACTIVITY_COMPOSE,
        compose_bom = versions::COMPOSE_BOM,
        jna = versions::JNA,
        coroutines = versions::KOTLINX_COROUTINES,
        junit = versions::JUNIT4,
        test_ext_junit = versions::ANDROIDX_TEST_EXT_JUNIT,
        test_runner = versions::ANDROIDX_TEST_RUNNER,
    )
}

pub fn root_build_gradle_kts() -> String {
    r#"plugins {
    alias(libs.plugins.android.application) apply false
    // O AGP 9 compila Kotlin nativamente; declarar o plugin aqui só fixa a
    // versão do Kotlin (a do catálogo) usada pelo build.
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
"#
    .to_string()
}

pub fn app_build_gradle_kts(ctx: &TemplateContext) -> String {
    format!(
        r#"plugins {{
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}}

android {{
    namespace = "{package_id}"
    compileSdk = {compile_sdk}

    defaultConfig {{
        applicationId = "{package_id}"
        minSdk = {min_sdk}
        targetSdk = {target_sdk}
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }}

    buildTypes {{
        release {{
            isMinifyEnabled = false
        }}
    }}

    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_{jvm}
        targetCompatibility = JavaVersion.VERSION_{jvm}
    }}

    buildFeatures {{
        compose = true
    }}

    sourceSets {{
        getByName("main") {{
            // Bindings Kotlin gerados pelo UniFFI (scripts/build-android-ffi.sh)
            kotlin.directories += "build/generated/uniffi"
            jniLibs.directories += "src/main/jniLibs"
        }}
    }}
}}

// Núcleo Rust: scripts/build-android-ffi.sh compila as .so (cargo-ndk) e gera os
// bindings Kotlin antes de cada build. ABIs: -Pjirau.rustAbis=x86_64 (a CLI
// passa só a ABI do dispositivo alvo para acelerar o ciclo).
val rustAbis = providers.gradleProperty("jirau.rustAbis").orElse("arm64-v8a,x86_64")
val repoRoot = rootProject.layout.projectDirectory.dir("..").asFile

fun registerRustCoreTask(taskName: String, profile: String) =
    tasks.register<Exec>(taskName) {{
        group = "rust"
        description = "Compila o núcleo Rust ($profile) e gera os bindings Kotlin"
        workingDir = repoRoot
        commandLine(
            listOf("bash", "scripts/build-android-ffi.sh", profile) +
                rustAbis.get().split(",").map {{ it.trim() }}.filter {{ it.isNotEmpty() }}
        )
    }}

val buildRustCoreDebug = registerRustCoreTask("buildRustCoreDebug", "debug")
val buildRustCoreRelease = registerRustCoreTask("buildRustCoreRelease", "release")

tasks.configureEach {{
    when (name) {{
        "preDebugBuild" -> dependsOn(buildRustCoreDebug)
        "preReleaseBuild" -> dependsOn(buildRustCoreRelease)
    }}
}}

dependencies {{
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    // UniFFI usa JNA para chamar a biblioteca Rust; o AAR traz o libjnidispatch.so.
    implementation("net.java.dev.jna:jna:${{libs.versions.jna.get()}}@aar")
    implementation(libs.kotlinx.coroutines.core)

    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)

    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)
}}
"#,
        package_id = ctx.package_id,
        compile_sdk = versions::ANDROID_COMPILE_SDK,
        min_sdk = versions::ANDROID_MIN_SDK,
        target_sdk = versions::ANDROID_TARGET_SDK,
        jvm = versions::ANDROID_JVM_TARGET,
    )
}

pub fn app_android_manifest_xml(ctx: &TemplateContext) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <application
        android:allowBackup="{allow_backup}"
        android:icon="@android:drawable/sym_def_app_icon"
        android:label="{name}"
        android:roundIcon="@android:drawable/sym_def_app_icon"
        android:supportsRtl="true"
        android:theme="@android:style/Theme.Material.Light.NoActionBar">
        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:theme="@android:style/Theme.Material.Light.NoActionBar">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>

</manifest>
"#,
        name = ctx.name,
        // O cofre não entra em backup: a chave fica no Android Keystore deste
        // aparelho e não acompanharia os dados restaurados.
        allow_backup = if ctx.app_template == "vault" {
            "false"
        } else {
            "true"
        }
    )
}

pub fn app_main_activity_kt(ctx: &TemplateContext) -> String {
    r#"package __PACKAGE__

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
import __PACKAGE__.ui.theme.JirauTheme
import __PACKAGE__.uniffi.AppEngine
import __PACKAGE__.uniffi.NativeException
import __PACKAGE__.uniffi.nativeGreet
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
        Text("🪵 __NAME__", style = MaterialTheme.typography.headlineMedium)
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
"#
    .replace("__PACKAGE__", &ctx.package_id)
    .replace("__NAME__", &ctx.name)
}

pub fn theme_color_kt(ctx: &TemplateContext) -> String {
    format!(
        r#"package {package_id}.ui.theme

import androidx.compose.ui.graphics.Color

val Purple80 = Color(0xFFD0BCFF)
val PurpleGrey80 = Color(0xFFCCC2DC)
val Pink80 = Color(0xFFEFB8C8)

val Purple40 = Color(0xFF6650a4)
val PurpleGrey40 = Color(0xFF625b71)
val Pink40 = Color(0xFF7D5260)

// Jirau Regional Accents
val JirauForestGreen = Color(0xFF1E5631)
val JirauRiverBlue = Color(0xFF2A52BE)
val JirauEarthWarm = Color(0xFFD27D2D)
"#,
        package_id = ctx.package_id
    )
}

pub fn theme_type_kt(ctx: &TemplateContext) -> String {
    format!(
        r#"package {package_id}.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

val Typography = Typography(
    bodyLarge = TextStyle(
        fontFamily = FontFamily.Default,
        fontWeight = FontWeight.Normal,
        fontSize = 16.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.5.sp
    ),
    titleLarge = TextStyle(
        fontFamily = FontFamily.Default,
        fontWeight = FontWeight.Bold,
        fontSize = 22.sp,
        lineHeight = 28.sp,
        letterSpacing = 0.sp
    )
)
"#,
        package_id = ctx.package_id
    )
}

pub fn theme_theme_kt(ctx: &TemplateContext) -> String {
    format!(
        r#"package {package_id}.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

private val DarkColorScheme = darkColorScheme(
    primary = Purple80,
    secondary = PurpleGrey80,
    tertiary = Pink80
)

private val LightColorScheme = lightColorScheme(
    primary = Purple40,
    secondary = PurpleGrey40,
    tertiary = Pink40
)

@Composable
fun JirauTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit
) {{
    val colorScheme = when {{
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {{
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }}
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }}

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}}
"#,
        package_id = ctx.package_id
    )
}

pub fn android_lifecycle_test_kt(ctx: &TemplateContext) -> String {
    r#"package __PACKAGE__

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** ViewModel e UI contra o núcleo Rust real (carregado via JNA/UniFFI). */
@RunWith(AndroidJUnit4::class)
class NativeLifecycleTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun greetingComesFromRust() {
        assertTrue(MainViewModel().status.value.contains("Android"))
    }

    @Test
    fun counterStateLivesInRust() {
        val vm = MainViewModel()
        assertEquals(0, vm.counter.value)
        vm.increment()
        vm.increment()
        assertEquals(2, vm.counter.value)
        vm.decrement()
        assertEquals(1, vm.counter.value)
    }

    @Test
    fun rustRejectsNegativeCounter() {
        val vm = MainViewModel()
        vm.decrement()
        assertEquals(0, vm.counter.value)
        assertTrue(vm.status.value.contains("não pode ficar negativo"))
    }

    @Test
    fun buttonsDriveTheRustEngine() {
        compose.setContent { AppScreen(MainViewModel()) }
        compose.onNodeWithTag("increment").performClick()
        compose.onNodeWithTag("increment").performClick()
        compose.onNodeWithTag("decrement").performClick()
        compose.onNodeWithTag("counter").assertTextEquals("1")
    }
}
"#
    .replace("__PACKAGE__", &ctx.package_id)
}
