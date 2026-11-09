plugins {
    alias(libs.plugins.android.application) apply false
    // O AGP 9 compila Kotlin nativamente; declarar o plugin aqui só fixa a
    // versão do Kotlin (a do catálogo) usada pelo build.
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
