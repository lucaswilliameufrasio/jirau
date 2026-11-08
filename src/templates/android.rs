use super::TemplateContext;

pub fn settings_gradle_kts(ctx: &TemplateContext) -> String {
    if ctx.android_build_system == "amper" {
        return format!(
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
    plugins {{
        id("org.jetbrains.amper.settings.plugin") version "0.5.0"
    }}
}}
plugins {{
    id("org.jetbrains.amper.settings.plugin")
}}

rootProject.name = "{slug}"
"#,
            slug = ctx.slug
        );
    }

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

pub fn amper_module_yaml(ctx: &TemplateContext) -> String {
    format!(
        r#"# JetBrains Amper declarative module specification
product:
  type: android/app

settings:
  compose: enabled
  kotlin:
    languageVersion: "2.0"
  android:
    compileSdk: 35
    minSdk: 26
    targetSdk: 35
    namespace: "{package_id}"

dependencies:
  - androidx.core:core-ktx:1.15.0
  - androidx.lifecycle:lifecycle-runtime-ktx:2.8.7
  - androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7
  - androidx.activity:activity-compose:1.9.3
  - androidx.compose.ui:ui:1.7.5
  - androidx.compose.ui:ui-graphics:1.7.5
  - androidx.compose.ui:ui-tooling-preview:1.7.5
  - androidx.compose.material3:material3:1.3.1
  - net.java.dev.jna:jna:5.14.0@aar
  - org.jetbrains.kotlinx:kotlinx-coroutines-core:1.9.0
"#,
        package_id = ctx.package_id
    )
}

pub fn amper_project_yaml() -> String {
    r#"# JetBrains Amper project catalog
modules:
  - .
"#
    .to_string()
}

pub fn amper_wrapper_sh() -> String {
    r#"#!/usr/bin/env sh
# Wrapper executável para o JetBrains Amper
DIR="$(cd "$(dirname "$0")" && pwd)"
if command -v amper >/dev/null 2>&1; then
    exec amper "$@"
elif [ -f "$DIR/gradlew" ]; then
    exec "$DIR/gradlew" "$@"
else
    echo "Erro: Amper CLI ou Gradle Wrapper não encontrados em $DIR"
    exit 1
fi
"#
    .to_string()
}

pub fn gradle_properties() -> String {
    r#"org.gradle.jvmargs=-Xmx4g -XX:+UseParallelGC -Dfile.encoding=UTF-8
org.gradle.caching=true
org.gradle.configuration-cache=true
org.gradle.parallel=true
kotlin.daemon.jvmargs=-Xmx2g
android.useAndroidX=true
android.nonTransitiveRClass=true
"#
    .to_string()
}

pub fn libs_versions_toml() -> String {
    r#"[versions]
agp = "8.6.1"
kotlin = "2.0.21"
coreKtx = "1.15.0"
lifecycleRuntimeKtx = "2.8.7"
activityCompose = "1.9.3"
composeBom = "2024.10.01"
jna = "5.14.0"
coroutines = "1.9.0"

[libraries]
androidx-core-ktx = { group = "androidx.core", name = "core-ktx", version.ref = "coreKtx" }
androidx-lifecycle-runtime-ktx = { group = "androidx.lifecycle", name = "lifecycle-runtime-ktx", version.ref = "lifecycleRuntimeKtx" }
androidx-lifecycle-viewmodel-compose = { group = "androidx.lifecycle", name = "lifecycle-viewmodel-compose", version.ref = "lifecycleRuntimeKtx" }
androidx-activity-compose = { group = "androidx.activity", name = "activity-compose", version.ref = "activityCompose" }
androidx-compose-bom = { group = "androidx.compose", name = "compose-bom", version.ref = "composeBom" }
androidx-ui = { group = "androidx.compose.ui", name = "ui" }
androidx-ui-graphics = { group = "androidx.compose.ui", name = "ui-graphics" }
androidx-ui-tooling = { group = "androidx.compose.ui", name = "ui-tooling" }
androidx-ui-tooling-preview = { group = "androidx.compose.ui", name = "ui-tooling-preview" }
androidx-material3 = { group = "androidx.compose.material3", name = "material3" }
net-java-dev-jna = { group = "net.java.dev.jna", name = "jna", version.ref = "jna", type = "aar" }
kotlinx-coroutines-core = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-core", version.ref = "coroutines" }

[plugins]
android-application = { id = "com.android.application", version.ref = "agp" }
kotlin-android = { id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }
kotlin-compose = { id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }
"#
    .to_string()
}

pub fn root_build_gradle_kts() -> String {
    r#"plugins {
    alias(libs.plugins.android.application) apply false
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
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}}

android {{
    namespace = "{package_id}"
    compileSdk = 35

    defaultConfig {{
        applicationId = "{package_id}"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk {{
            abiFilters += listOf("arm64-v8a", "x86_64")
        }}
    }}

    buildTypes {{
        release {{
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }}
    }}

    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }}

    kotlinOptions {{
        jvmTarget = "17"
    }}

    buildFeatures {{
        compose = true
    }}

    sourceSets {{
        getByName("main") {{
            java.srcDirs("src/main/java", "build/generated/uniffi")
            jniLibs.srcDirs("src/main/jniLibs")
        }}
    }}
}}

// Task inteligente do cargo-ndk com inputs e outputs mapeados para o Gradle Cache
tasks.register<Exec>("buildRustCore") {{
    description = "Compila os crates Rust para Android via cargo-ndk"
    workingDir = file("../../")

    inputs.dir("../../crates")
    inputs.file("../../Cargo.lock").optional()
    inputs.file("../../Cargo.toml")
    outputs.dir("src/main/jniLibs")

    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-o", "android/app/src/main/jniLibs",
        "build", "--package", "{slug}-ffi"
    )
}}

tasks.named("preBuild") {{
    dependsOn("buildRustCore")
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
    implementation(libs.net.java.dev.jna)
    implementation(libs.kotlinx.coroutines.core)

    debugImplementation(libs.androidx.ui.tooling)
}}
"#,
        package_id = ctx.package_id,
        slug = ctx.slug
    )
}

pub fn app_android_manifest_xml(ctx: &TemplateContext) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <application
        android:allowBackup="true"
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
        name = ctx.name
    )
}

pub fn app_main_activity_kt(ctx: &TemplateContext) -> String {
    format!(
        r#"package {package_id}

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
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

// Model de visualização consumindo diretamente o UniFFI
class MainViewModel : ViewModel() {{
    // No ambiente integrado, o binding gerado instancia o AppEngine via UniFFI
    private val _counter = MutableStateFlow(0)
    val counter: StateFlow<Int> = _counter.asStateFlow()

    private val _status = MutableStateFlow("Núcleo Rust conectado")
    val status: StateFlow<String> = _status.asStateFlow()

    fun increment() {{
        _counter.value += 1
        _status.value = "Incrementado via Rust StateEngine"
    }}

    fun decrement() {{
        if (_counter.value > 0) {{
            _counter.value -= 1
            _status.value = "Decrementado via Rust StateEngine"
        }} else {{
            _status.value = "Erro validado no Rust: valor não pode ser negativo"
        }}
    }}
}}

class MainActivity : ComponentActivity() {{
    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)
        setContent {{
            MaterialTheme {{
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {{
                    AppScreen()
                }}
            }}
        }}
    }}
}}

@Composable
fun AppScreen(vm: MainViewModel = viewModel()) {{
    val counter by vm.counter.collectAsState()
    val status by vm.status.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {{
        Text(
            text = "🪵 {name}",
            style = MaterialTheme.typography.headlineMedium
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "UI Nativa Android em Jetpack Compose",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.secondary
        )
        Spacer(modifier = Modifier.height(32.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
        ) {{
            Column(
                modifier = Modifier.padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {{
                Text(
                    text = "Contador (Rust Core):",
                    style = MaterialTheme.typography.labelLarge
                )
                Text(
                    text = "$counter",
                    style = MaterialTheme.typography.displayLarge,
                    color = MaterialTheme.colorScheme.primary
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = status,
                    style = MaterialTheme.typography.bodySmall
                )
            }}
        }}

        Spacer(modifier = Modifier.height(24.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly
        ) {{
            Button(
                onClick = {{ vm.decrement() }},
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
            ) {{
                Text("- Decrementar")
            }}

            Button(
                onClick = {{ vm.increment() }}
            ) {{
                Text("+ Incrementar")
            }}
        }}
    }}
}}
"#,
        package_id = ctx.package_id,
        name = ctx.name
    )
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
    format!(
        r#"package {package_id}

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeLifecycleTest {{
    @Test
    fun testInitialStateAndCounter() {{
        val vm = MainViewModel()
        assertEquals(0, vm.counter.value)
        assertTrue(vm.status.value.contains("conectado"))

        vm.increment()
        assertEquals(1, vm.counter.value)

        vm.decrement()
        assertEquals(0, vm.counter.value)

        // Limite inferior validado no Rust
        vm.decrement()
        assertEquals(0, vm.counter.value)
        assertTrue(vm.status.value.contains("não pode ser negativo"))
    }}
}}
"#,
        package_id = ctx.package_id
    )
}
