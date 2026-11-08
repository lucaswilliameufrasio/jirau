plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.example.showcase"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.example.showcase"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
    }

    sourceSets {
        getByName("main") {
            java.srcDirs("src/main/java", "build/generated/uniffi")
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

// Task inteligente do cargo-ndk com inputs e outputs mapeados para o Gradle Cache
tasks.register<Exec>("buildRustCore") {
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
        "build", "--package", "all-platforms-showcase-ffi"
    )
}

tasks.named("preBuild") {
    dependsOn("buildRustCore")
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
    implementation(libs.net.java.dev.jna)
    implementation(libs.kotlinx.coroutines.core)

    debugImplementation(libs.androidx.ui.tooling)
}
