pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
    plugins {
        id("org.jetbrains.amper.settings.plugin") version "0.5.0"
    }
}
plugins {
    id("org.jetbrains.amper.settings.plugin")
}

rootProject.name = "all-platforms-showcase"
