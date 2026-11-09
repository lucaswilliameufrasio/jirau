plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "io.github.jirau.vault"
    compileSdk = 37

    defaultConfig {
        applicationId = "io.github.jirau.vault"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    buildFeatures {
        compose = true
    }

    sourceSets {
        getByName("main") {
            // Bindings Kotlin gerados pelo UniFFI (scripts/build-android-ffi.sh)
            kotlin.directories += "build/generated/uniffi"
            jniLibs.directories += "src/main/jniLibs"
        }
    }
}

// Núcleo Rust: scripts/build-android-ffi.sh compila as .so (cargo-ndk) e gera os
// bindings Kotlin antes de cada build. ABIs: -Pjirau.rustAbis=x86_64 (a CLI
// passa só a ABI do dispositivo alvo para acelerar o ciclo).
val rustAbis = providers.gradleProperty("jirau.rustAbis").orElse("arm64-v8a,x86_64")
val repoRoot = rootProject.layout.projectDirectory.dir("..").asFile

fun registerRustCoreTask(taskName: String, profile: String) =
    tasks.register<Exec>(taskName) {
        group = "rust"
        description = "Compila o núcleo Rust ($profile) e gera os bindings Kotlin"
        workingDir = repoRoot
        commandLine(
            listOf("bash", "scripts/build-android-ffi.sh", profile) +
                rustAbis.get().split(",").map { it.trim() }.filter { it.isNotEmpty() }
        )
    }

val buildRustCoreDebug = registerRustCoreTask("buildRustCoreDebug", "debug")
val buildRustCoreRelease = registerRustCoreTask("buildRustCoreRelease", "release")

tasks.configureEach {
    when (name) {
        "preDebugBuild" -> dependsOn(buildRustCoreDebug)
        "preReleaseBuild" -> dependsOn(buildRustCoreRelease)
    }
}

dependencies {
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
    implementation("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
    implementation(libs.kotlinx.coroutines.core)

    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)

    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)
}
