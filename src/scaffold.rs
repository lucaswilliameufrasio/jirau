use crate::templates::android;
use crate::templates::desktop;
use crate::templates::ios;
use crate::templates::root;
use crate::templates::rust_core;
use crate::templates::rust_ffi;
use crate::templates::TemplateContext;
use crate::ui;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct Scaffolder;

impl Scaffolder {
    pub fn create_project(ctx: &TemplateContext, destination: &Path) -> Result<()> {
        let project_dir = destination.join(&ctx.slug);

        if project_dir.exists() {
            anyhow::bail!(
                "O diretório '{}' já existe. Escolha outro nome ou remova o diretório antes de continuar.",
                project_dir.display()
            );
        }

        let spinner = ui::create_spinner("Gerando estrutura do projeto jirau...");

        // 1. Root files
        fs::create_dir_all(&project_dir)
            .with_context(|| format!("Falha ao criar diretório {}", project_dir.display()))?;

        fs::write(project_dir.join("Cargo.toml"), root::root_cargo_toml(ctx))?;
        fs::write(project_dir.join(".gitignore"), root::root_gitignore())?;
        fs::write(project_dir.join(".mise.toml"), root::root_mise_toml())?;
        fs::write(project_dir.join("Makefile"), root::root_makefile(ctx))?;
        fs::write(
            project_dir.join("TESTING_STANDARD.md"),
            root::testing_standard_md(ctx),
        )?;
        fs::write(project_dir.join("jirau.toml"), root::jirau_toml(ctx))?;

        // 2. crates/<slug>-core
        let core_dir = project_dir
            .join("crates")
            .join(format!("{}-core", ctx.slug));
        fs::create_dir_all(core_dir.join("src"))?;
        fs::write(core_dir.join("Cargo.toml"), rust_core::core_cargo_toml(ctx))?;
        fs::write(core_dir.join("src/lib.rs"), rust_core::core_lib_rs(ctx))?;

        // 3. crates/<slug>-ffi
        let ffi_dir = project_dir.join("crates").join(format!("{}-ffi", ctx.slug));
        fs::create_dir_all(ffi_dir.join("src"))?;
        fs::create_dir_all(ffi_dir.join("tests"))?;
        fs::write(ffi_dir.join("Cargo.toml"), rust_ffi::ffi_cargo_toml(ctx))?;
        fs::write(ffi_dir.join("build.rs"), rust_ffi::ffi_build_rs())?;
        fs::write(ffi_dir.join("uniffi.toml"), rust_ffi::ffi_uniffi_toml(ctx))?;
        fs::write(ffi_dir.join("src/lib.rs"), rust_ffi::ffi_lib_rs(ctx))?;
        fs::write(
            ffi_dir.join("tests/lifecycle_integration_tests.rs"),
            rust_ffi::ffi_lifecycle_test_rs(ctx),
        )?;

        // 4. Android
        if ctx.platforms.contains(&"android".to_string()) {
            let android_dir = project_dir.join("android");
            let app_dir = android_dir.join("app");
            let pkg_src_dir = app_dir.join("src/main/java").join(&ctx.package_path);
            let theme_dir = pkg_src_dir.join("ui/theme");
            let test_src_dir = app_dir.join("src/androidTest/java").join(&ctx.package_path);
            let jni_libs_dir = app_dir.join("src/main/jniLibs");
            let gradle_toml_dir = android_dir.join("gradle");

            fs::create_dir_all(&pkg_src_dir)?;
            fs::create_dir_all(&theme_dir)?;
            fs::create_dir_all(&test_src_dir)?;
            fs::create_dir_all(&jni_libs_dir)?;
            fs::create_dir_all(&gradle_toml_dir)?;

            fs::write(
                android_dir.join("settings.gradle.kts"),
                android::settings_gradle_kts(ctx),
            )?;
            fs::write(
                android_dir.join("gradle.properties"),
                android::gradle_properties(),
            )?;
            fs::write(
                android_dir.join("build.gradle.kts"),
                android::root_build_gradle_kts(),
            )?;
            fs::write(
                gradle_toml_dir.join("libs.versions.toml"),
                android::libs_versions_toml(),
            )?;

            fs::write(
                app_dir.join("build.gradle.kts"),
                android::app_build_gradle_kts(ctx),
            )?;
            fs::write(
                app_dir.join("src/main/AndroidManifest.xml"),
                android::app_android_manifest_xml(ctx),
            )?;
            fs::write(
                pkg_src_dir.join("MainActivity.kt"),
                android::app_main_activity_kt(ctx),
            )?;
            fs::write(theme_dir.join("Color.kt"), android::theme_color_kt(ctx))?;
            fs::write(theme_dir.join("Type.kt"), android::theme_type_kt(ctx))?;
            fs::write(theme_dir.join("Theme.kt"), android::theme_theme_kt(ctx))?;
            fs::write(
                test_src_dir.join("NativeLifecycleTest.kt"),
                android::android_lifecycle_test_kt(ctx),
            )?;
            fs::write(jni_libs_dir.join(".gitkeep"), "")?;

            if ctx.android_build_system == "amper" {
                fs::write(
                    android_dir.join("module.yaml"),
                    android::amper_module_yaml(ctx),
                )?;
                fs::write(
                    android_dir.join("project.yaml"),
                    android::amper_project_yaml(),
                )?;
                let amper_script = android_dir.join("amper");
                fs::write(&amper_script, android::amper_wrapper_sh())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&amper_script, fs::Permissions::from_mode(0o755));
                }
            }

            // Documentação de setup e scripts de ambiente Android
            let docs_dir = project_dir.join("docs");
            let scripts_dir = project_dir.join("scripts");
            fs::create_dir_all(&docs_dir)?;
            fs::create_dir_all(&scripts_dir)?;
            fs::write(
                docs_dir.join("setup-android.md"),
                root::setup_android_md(ctx),
            )?;
            let setup_script = scripts_dir.join("setup-android-env.sh");
            fs::write(&setup_script, root::setup_android_env_sh(ctx))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&setup_script, fs::Permissions::from_mode(0o755));
            }
        }

        // 5. iOS
        if ctx.platforms.contains(&"ios".to_string()) {
            let ios_dir = project_dir.join("ios");
            let sources_dir = ios_dir.join("App/Sources");
            let tests_dir = ios_dir.join("App/Tests");
            let gen_dir = ios_dir.join("Generated");

            fs::create_dir_all(&sources_dir)?;
            fs::create_dir_all(&tests_dir)?;
            fs::create_dir_all(&gen_dir)?;

            fs::write(ios_dir.join("project.yml"), ios::project_yml(ctx))?;
            fs::write(
                sources_dir.join("ContentView.swift"),
                ios::content_view_swift(ctx),
            )?;
            fs::write(
                sources_dir.join(format!("{}App.swift", ctx.pascal_name)),
                ios::app_swift(ctx),
            )?;
            fs::write(sources_dir.join("Theme.swift"), ios::theme_swift())?;
            fs::write(
                tests_dir.join("NativeLifecycleTests.swift"),
                ios::lifecycle_tests_swift(ctx),
            )?;
            fs::write(gen_dir.join(".gitkeep"), "")?;
        }

        // 6. Desktop (Rune UI)
        if ctx.platforms.contains(&"desktop".to_string()) {
            let desktop_dir = project_dir
                .join("crates")
                .join(format!("{}-desktop", ctx.slug));
            fs::create_dir_all(desktop_dir.join("src"))?;
            fs::write(
                desktop_dir.join("Cargo.toml"),
                desktop::desktop_cargo_toml(ctx),
            )?;
            fs::write(
                desktop_dir.join("src/main.rs"),
                desktop::desktop_main_rs(ctx),
            )?;
        }

        ui::finish_spinner_success(
            &spinner,
            &format!("Projeto '{}' criado com sucesso!", ctx.name),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scaffolding_generates_all_files() {
        let temp = tempdir().unwrap();
        let ctx = TemplateContext::new(
            "Acari App",
            "com.lucas.acari",
            vec![
                "android".to_string(),
                "ios".to_string(),
                "desktop".to_string(),
            ],
        );

        Scaffolder::create_project(&ctx, temp.path()).unwrap();

        let root = temp.path().join("acari-app");
        assert!(root.join("Cargo.toml").exists());
        assert!(root.join(".gitignore").exists());
        assert!(root.join(".mise.toml").exists());
        assert!(root.join("Makefile").exists());
        assert!(root.join("TESTING_STANDARD.md").exists());
        assert!(root.join("jirau.toml").exists());

        // Crates
        assert!(root.join("crates/acari-app-core/Cargo.toml").exists());
        assert!(root.join("crates/acari-app-core/src/lib.rs").exists());
        assert!(root.join("crates/acari-app-ffi/Cargo.toml").exists());
        assert!(root.join("crates/acari-app-ffi/uniffi.toml").exists());
        assert!(root.join("crates/acari-app-ffi/build.rs").exists());
        assert!(root.join("crates/acari-app-ffi/src/lib.rs").exists());
        assert!(root
            .join("crates/acari-app-ffi/tests/lifecycle_integration_tests.rs")
            .exists());
        assert!(root.join("crates/acari-app-desktop/Cargo.toml").exists());
        assert!(root.join("crates/acari-app-desktop/src/main.rs").exists());

        // Android
        assert!(root.join("android/settings.gradle.kts").exists());
        assert!(root.join("android/gradle.properties").exists());
        assert!(root.join("android/gradle/libs.versions.toml").exists());
        assert!(root.join("android/app/build.gradle.kts").exists());
        assert!(root
            .join("android/app/src/main/AndroidManifest.xml")
            .exists());
        assert!(root
            .join("android/app/src/main/java/com/lucas/acari/MainActivity.kt")
            .exists());
        assert!(root
            .join("android/app/src/main/java/com/lucas/acari/ui/theme/Theme.kt")
            .exists());
        assert!(root
            .join("android/app/src/androidTest/java/com/lucas/acari/NativeLifecycleTest.kt")
            .exists());
        assert!(root.join("docs/setup-android.md").exists());
        assert!(root.join("scripts/setup-android-env.sh").exists());

        // iOS
        assert!(root.join("ios/project.yml").exists());
        assert!(root.join("ios/App/Sources/ContentView.swift").exists());
        assert!(root.join("ios/App/Sources/AcariAppApp.swift").exists());
        assert!(root.join("ios/App/Sources/Theme.swift").exists());
        assert!(root
            .join("ios/App/Tests/NativeLifecycleTests.swift")
            .exists());

        // Duplicate protection
        assert!(Scaffolder::create_project(&ctx, temp.path()).is_err());
    }

    #[test]
    fn test_scaffolding_with_sqlite_and_amper() {
        let temp = tempdir().unwrap();
        let ctx = TemplateContext::with_options(
            "Tucupi App",
            "com.lucas.tucupi",
            vec!["android".to_string()],
            "sqlite",
            "amper",
            "0.2.0",
        );

        Scaffolder::create_project(&ctx, temp.path()).unwrap();
        let root = temp.path().join("tucupi-app");

        // Amper checks
        assert!(root.join("android/module.yaml").exists());
        assert!(root.join("android/project.yaml").exists());
        assert!(root.join("android/amper").exists());

        // SQLite checks
        let core_cargo =
            std::fs::read_to_string(root.join("crates/tucupi-app-core/Cargo.toml")).unwrap();
        assert!(core_cargo.contains("rusqlite"));

        let core_lib =
            std::fs::read_to_string(root.join("crates/tucupi-app-core/src/lib.rs")).unwrap();
        assert!(core_lib.contains("StorageEngine"));
        assert!(core_lib.contains("kv_store"));

        let jirau_toml = std::fs::read_to_string(root.join("jirau.toml")).unwrap();
        assert!(jirau_toml.contains("storage = \"sqlite\""));
        assert!(jirau_toml.contains("android_build_system = \"amper\""));
        assert!(jirau_toml.contains("template_version = \"0.2.0\""));
    }
}
