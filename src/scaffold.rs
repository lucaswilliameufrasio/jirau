use crate::templates::TemplateContext;
use crate::templates::android;
use crate::templates::desktop;
use crate::templates::ios;
use crate::templates::root;
use crate::templates::rust_core;
use crate::templates::rust_ffi;
use crate::templates::vault;
use crate::ui;
use anyhow::{Context, Result};
use rust_i18n::t;
use std::fs;
use std::path::{Path, PathBuf};

/// Diretório (versionado no git) com a cópia exata do que o template gerou.
/// É a "base" do 3-way merge do `jirau upgrade`.
pub const TEMPLATE_BASE_DIR: &str = ".jirau/template-base";

/// Grava arquivos do projeto e registra o que foi gerado pelo template.
pub struct ProjectWriter {
    root: PathBuf,
    written: Vec<(PathBuf, Vec<u8>)>,
}

impl ProjectWriter {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            written: Vec::new(),
        }
    }

    pub fn write(&mut self, rel: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
        let rel = rel.as_ref();
        let formatted;
        let contents = if rel.extension().is_some_and(|e| e == "rs") {
            formatted = format_rust(contents.as_ref());
            formatted.as_slice()
        } else {
            contents.as_ref()
        };
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)
            .with_context(|| t!("scaffold.write_failed", path = path.display()).into_owned())?;
        self.written.push((rel.to_path_buf(), contents.to_vec()));
        Ok(())
    }

    pub fn write_executable(
        &mut self,
        rel: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
    ) -> Result<()> {
        let rel = rel.as_ref().to_path_buf();
        self.write(&rel, contents)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(self.root.join(&rel), fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }

    /// Só cria o arquivo se ele ainda não existir (não registra na base).
    pub fn write_if_missing(&mut self, rel: impl AsRef<Path>, contents: &str) -> Result<()> {
        if self.root.join(rel.as_ref()).exists() {
            return Ok(());
        }
        self.write(rel, contents)
    }

    /// Salva a cópia do que foi gerado em `.jirau/template-base/`.
    pub fn save_template_base(&self) -> Result<()> {
        let base = self.root.join(TEMPLATE_BASE_DIR);
        for (rel, contents) in &self.written {
            let path = base.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, contents)?;
        }
        Ok(())
    }
}

/// Formata código Rust gerado com o `rustfmt` da toolchain, para que o projeto
/// já nasça passando em `cargo fmt --check` (exigido pelo CI gerado). Sem
/// rustfmt disponível o conteúdo é mantido como está.
fn format_rust(source: &[u8]) -> Vec<u8> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let child = Command::new("rustfmt")
        .args([
            "--edition",
            crate::templates::versions::RUST_EDITION,
            "--emit",
            "stdout",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return source.to_vec();
    };
    if let Some(mut stdin) = child.stdin.take()
        && stdin.write_all(source).is_err()
    {
        let _ = child.kill();
        return source.to_vec();
    }
    match child.wait_with_output() {
        Ok(out) if out.status.success() && !out.stdout.is_empty() => out.stdout,
        _ => source.to_vec(),
    }
}

pub struct Scaffolder;

impl Scaffolder {
    pub fn create_project(ctx: &TemplateContext, destination: &Path) -> Result<()> {
        let project_dir = destination.join(&ctx.slug);

        if project_dir.exists() {
            anyhow::bail!(
                "{}",
                t!("scaffold.dir_exists", path = project_dir.display())
            );
        }

        let spinner = ui::create_spinner(&t!("scaffold.generating"));
        fs::create_dir_all(&project_dir).with_context(|| {
            t!("scaffold.create_dir_failed", path = project_dir.display()).into_owned()
        })?;

        let mut w = ProjectWriter::new(&project_dir);
        Self::write_project_files(ctx, &mut w)?;
        w.save_template_base()?;

        ui::finish_spinner_success(&spinner, &t!("scaffold.created", name = ctx.name));
        Ok(())
    }

    /// Todos os arquivos do template (sem o snapshot da base).
    pub fn write_project_files(ctx: &TemplateContext, w: &mut ProjectWriter) -> Result<()> {
        // 1. Raiz
        w.write("Cargo.toml", root::root_cargo_toml(ctx))?;
        w.write(".gitignore", root::root_gitignore())?;
        w.write(".mise.toml", root::root_mise_toml())?;
        w.write("Makefile", root::root_makefile(ctx))?;
        // Docs em inglês (padrão) e português.
        w.write("TESTING_STANDARD.md", root::testing_standard_md_en(ctx))?;
        w.write(
            "TESTING_STANDARD.pt-BR.md",
            root::testing_standard_md_pt_br(ctx),
        )?;
        w.write("jirau.toml", root::jirau_toml(ctx))?;
        if ctx.ci_workflow {
            w.write(".github/workflows/ci.yml", root::root_ci_workflow(ctx))?;
        }

        // 2. crates/<slug>-core
        let vault = ctx.app_template == "vault";
        let core = format!("crates/{}-core", ctx.slug);
        if vault {
            w.write(
                format!("{core}/Cargo.toml"),
                vault::vault_core_cargo_toml(ctx),
            )?;
            w.write(format!("{core}/src/lib.rs"), vault::vault_core_lib_rs(ctx))?;
        } else {
            w.write(
                format!("{core}/Cargo.toml"),
                rust_core::core_cargo_toml(ctx),
            )?;
            w.write(format!("{core}/src/lib.rs"), rust_core::core_lib_rs(ctx))?;
        }

        // 3. crates/<slug>-ffi
        let ffi = format!("crates/{}-ffi", ctx.slug);
        w.write(format!("{ffi}/Cargo.toml"), rust_ffi::ffi_cargo_toml(ctx))?;
        w.write(format!("{ffi}/uniffi.toml"), rust_ffi::ffi_uniffi_toml(ctx))?;
        w.write(
            format!("{ffi}/src/bin/uniffi-bindgen.rs"),
            rust_ffi::ffi_uniffi_bindgen_rs(),
        )?;
        if vault {
            w.write(format!("{ffi}/src/lib.rs"), vault::vault_ffi_lib_rs(ctx))?;
            w.write(
                format!("{ffi}/tests/lifecycle_integration_tests.rs"),
                vault::vault_ffi_lifecycle_test_rs(ctx),
            )?;
        } else {
            w.write(format!("{ffi}/src/lib.rs"), rust_ffi::ffi_lib_rs(ctx))?;
            w.write(
                format!("{ffi}/tests/lifecycle_integration_tests.rs"),
                rust_ffi::ffi_lifecycle_test_rs(ctx),
            )?;
        }

        if ctx.platforms.iter().any(|p| p == "android") {
            Self::write_android(ctx, w)?;
        }
        if ctx.platforms.iter().any(|p| p == "ios") {
            Self::write_ios(ctx, w)?;
        }
        if ctx.platforms.iter().any(|p| p == "desktop") {
            Self::write_desktop(ctx, w)?;
        }
        Ok(())
    }

    /// Plataforma Android (usado também por `jirau add android`).
    pub fn write_android(ctx: &TemplateContext, w: &mut ProjectWriter) -> Result<()> {
        let vault = ctx.app_template == "vault";
        let pkg = format!("android/app/src/main/java/{}", ctx.package_path);
        let test_pkg = format!("android/app/src/androidTest/java/{}", ctx.package_path);

        w.write(
            "android/settings.gradle.kts",
            android::settings_gradle_kts(ctx),
        )?;
        w.write("android/gradle.properties", android::gradle_properties())?;
        w.write("android/build.gradle.kts", android::root_build_gradle_kts())?;
        w.write(
            "android/gradle/libs.versions.toml",
            android::libs_versions_toml(),
        )?;
        w.write_executable("android/gradlew", android::GRADLEW)?;
        w.write("android/gradlew.bat", android::GRADLEW_BAT)?;
        w.write(
            "android/gradle/wrapper/gradle-wrapper.jar",
            android::GRADLE_WRAPPER_JAR,
        )?;
        w.write(
            "android/gradle/wrapper/gradle-wrapper.properties",
            android::gradle_wrapper_properties(),
        )?;
        w.write(
            "android/app/build.gradle.kts",
            android::app_build_gradle_kts(ctx),
        )?;
        w.write(
            "android/app/src/main/AndroidManifest.xml",
            android::app_android_manifest_xml(ctx),
        )?;
        if vault {
            w.write(
                format!("{pkg}/MainActivity.kt"),
                vault::vault_android_main_activity_kt(ctx),
            )?;
            w.write(
                format!("{pkg}/VaultCore.kt"),
                vault::vault_android_core_kt(ctx),
            )?;
            w.write(
                format!("{test_pkg}/NativeLifecycleTest.kt"),
                vault::vault_android_tests_kt(ctx),
            )?;
        } else {
            w.write(
                format!("{pkg}/MainActivity.kt"),
                android::app_main_activity_kt(ctx),
            )?;
            w.write(
                format!("{test_pkg}/NativeLifecycleTest.kt"),
                android::android_lifecycle_test_kt(ctx),
            )?;
        }
        w.write(
            format!("{pkg}/ui/theme/Color.kt"),
            android::theme_color_kt(ctx),
        )?;
        w.write(
            format!("{pkg}/ui/theme/Type.kt"),
            android::theme_type_kt(ctx),
        )?;
        w.write(
            format!("{pkg}/ui/theme/Theme.kt"),
            android::theme_theme_kt(ctx),
        )?;

        w.write_executable(
            "scripts/build-android-ffi.sh",
            android::build_android_ffi_sh(ctx),
        )?;
        w.write("docs/setup-android.md", root::setup_android_md_en(ctx))?;
        w.write(
            "docs/setup-android.pt-BR.md",
            root::setup_android_md_pt_br(ctx),
        )?;
        w.write_executable(
            "scripts/setup-android-env.sh",
            root::setup_android_env_sh(ctx),
        )?;
        Ok(())
    }

    /// Plataforma iOS (usado também por `jirau add ios`).
    pub fn write_ios(ctx: &TemplateContext, w: &mut ProjectWriter) -> Result<()> {
        let vault = ctx.app_template == "vault";
        w.write("ios/project.yml", ios::project_yml(ctx))?;
        if vault {
            w.write(
                "ios/App/Sources/ContentView.swift",
                vault::vault_ios_content_view_swift(ctx),
            )?;
            w.write(
                "ios/App/Tests/NativeLifecycleTests.swift",
                vault::vault_ios_tests_swift(ctx),
            )?;
        } else {
            w.write(
                "ios/App/Sources/ContentView.swift",
                ios::content_view_swift(ctx),
            )?;
            w.write(
                "ios/App/Tests/NativeLifecycleTests.swift",
                ios::lifecycle_tests_swift(ctx),
            )?;
        }
        w.write(
            format!("ios/App/Sources/{}App.swift", ctx.pascal_name),
            ios::app_swift(ctx),
        )?;
        w.write("ios/App/Sources/Theme.swift", ios::theme_swift())?;
        w.write("ios/Generated/.gitkeep", "")?;
        w.write_executable("scripts/build-ios-ffi.sh", ios::build_ios_ffi_sh(ctx))?;
        Ok(())
    }

    /// Plataforma Desktop em Rune (usado também por `jirau add desktop`).
    pub fn write_desktop(ctx: &TemplateContext, w: &mut ProjectWriter) -> Result<()> {
        let dir = format!("crates/{}-desktop", ctx.slug);
        w.write(
            format!("{dir}/Cargo.toml"),
            desktop::desktop_cargo_toml(ctx),
        )?;
        let main = if ctx.app_template == "vault" {
            vault::vault_desktop_main_rs(ctx)
        } else {
            desktop::desktop_main_rs(ctx)
        };
        w.write(format!("{dir}/src/main.rs"), main)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::versions;
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
        assert!(
            root.join("crates/acari-app-ffi/src/bin/uniffi-bindgen.rs")
                .exists()
        );
        assert!(root.join("crates/acari-app-ffi/src/lib.rs").exists());
        assert!(
            root.join("crates/acari-app-ffi/tests/lifecycle_integration_tests.rs")
                .exists()
        );
        assert!(root.join("crates/acari-app-desktop/Cargo.toml").exists());
        assert!(root.join("crates/acari-app-desktop/src/main.rs").exists());

        // Android
        assert!(root.join("android/settings.gradle.kts").exists());
        assert!(root.join("android/gradle.properties").exists());
        assert!(root.join("android/gradle/libs.versions.toml").exists());
        assert!(root.join("android/app/build.gradle.kts").exists());
        assert!(
            root.join("android/app/src/main/AndroidManifest.xml")
                .exists()
        );
        assert!(
            root.join("android/app/src/main/java/com/lucas/acari/MainActivity.kt")
                .exists()
        );
        assert!(
            root.join("android/app/src/main/java/com/lucas/acari/ui/theme/Theme.kt")
                .exists()
        );
        assert!(
            root.join("android/app/src/androidTest/java/com/lucas/acari/NativeLifecycleTest.kt")
                .exists()
        );
        assert!(root.join("android/gradlew").exists());
        assert!(
            root.join("android/gradle/wrapper/gradle-wrapper.jar")
                .exists()
        );
        assert!(root.join("scripts/build-android-ffi.sh").exists());
        assert!(root.join("docs/setup-android.md").exists());
        assert!(root.join("scripts/setup-android-env.sh").exists());

        // iOS
        assert!(root.join("ios/project.yml").exists());
        assert!(root.join("ios/App/Sources/ContentView.swift").exists());
        assert!(root.join("ios/App/Sources/AcariAppApp.swift").exists());
        assert!(root.join("ios/App/Sources/Theme.swift").exists());
        assert!(
            root.join("ios/App/Tests/NativeLifecycleTests.swift")
                .exists()
        );

        // Duplicate protection
        assert!(Scaffolder::create_project(&ctx, temp.path()).is_err());
    }

    #[test]
    fn test_scaffolding_with_sqlite() {
        let temp = tempdir().unwrap();
        let ctx = TemplateContext::with_options(
            "Tucupi App",
            "com.lucas.tucupi",
            vec!["android".to_string()],
            "sqlite",
            "gradle",
            "0.2.0",
        );

        Scaffolder::create_project(&ctx, temp.path()).unwrap();
        let root = temp.path().join("tucupi-app");

        let core_cargo =
            std::fs::read_to_string(root.join("crates/tucupi-app-core/Cargo.toml")).unwrap();
        assert!(core_cargo.contains(&format!(
            "rusqlite = {{ version = \"{}\"",
            versions::RUSQLITE
        )));

        let core_lib =
            std::fs::read_to_string(root.join("crates/tucupi-app-core/src/lib.rs")).unwrap();
        assert!(core_lib.contains("StorageEngine"));
        assert!(core_lib.contains("kv_store"));

        let jirau_toml = std::fs::read_to_string(root.join("jirau.toml")).unwrap();
        assert!(jirau_toml.contains("storage = \"sqlite\""));
        assert!(jirau_toml.contains("template_version = \"0.2.0\""));
    }

    #[test]
    fn test_generated_makefile_recipes_are_tab_indented() {
        let temp = tempdir().unwrap();
        let ctx = TemplateContext::new(
            "Make App",
            "com.example.make",
            vec!["android".to_string(), "ios".to_string()],
        );
        Scaffolder::create_project(&ctx, temp.path()).unwrap();
        let makefile = fs::read_to_string(temp.path().join("make-app/Makefile")).unwrap();

        let help_body: Vec<&str> = makefile
            .lines()
            .skip_while(|l| *l != "help:")
            .skip(1)
            .take_while(|l| !l.is_empty())
            .collect();
        assert!(help_body.len() > 5);
        for line in help_body {
            assert!(
                line.starts_with('\t'),
                "linha de receita sem TAB: {:?}",
                line
            );
        }
        assert!(makefile.contains("ios-ffi:\n\t@$(RUN) ./scripts/build-ios-ffi.sh debug"));
        assert!(makefile.contains("ios-project: ios-ffi"));
    }

    #[test]
    fn test_ios_scaffold_wires_rust_core_into_xcode() {
        let temp = tempdir().unwrap();
        let ctx = TemplateContext::new("Ponte App", "com.example.ponte", vec!["ios".to_string()]);
        Scaffolder::create_project(&ctx, temp.path()).unwrap();
        let root = temp.path().join("ponte-app");

        let script = root.join("scripts/build-ios-ffi.sh");
        assert!(script.exists());
        let script_body = fs::read_to_string(&script).unwrap();
        assert!(script_body.contains("CRATE=\"ponte-app-ffi\""));
        assert!(script_body.contains("LIB=\"ponte_app_ffi\""));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&script).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "script deve ser executável");
        }

        assert!(
            root.join("crates/ponte-app-ffi/src/bin/uniffi-bindgen.rs")
                .exists()
        );

        // XcodeGen só linka frameworks declarados em `dependencies`.
        let project_yml = fs::read_to_string(root.join("ios/project.yml")).unwrap();
        assert!(project_yml.contains("dependencies:\n      # Núcleo Rust"));
        assert!(
            project_yml.contains(
                "- framework: Frameworks/ponte_app_ffi.xcframework\n        embed: false"
            )
        );
        assert!(!project_yml.contains("\n    frameworks:"));
        assert!(project_yml.contains("GENERATE_INFOPLIST_FILE: YES"));
        assert!(project_yml.contains("PonteAppTests:"));

        // A UI chama o núcleo Rust em vez de simular o estado em Swift.
        let view = fs::read_to_string(root.join("ios/App/Sources/ContentView.swift")).unwrap();
        assert!(view.contains("AppEngine()"));
        assert!(view.contains("try engine.decrement()"));
    }

    #[test]
    fn test_android_scaffold_wires_rust_core_into_gradle() {
        let temp = tempdir().unwrap();
        let ctx = TemplateContext::new(
            "Ponte App",
            "com.example.ponte",
            vec!["android".to_string()],
        );
        Scaffolder::create_project(&ctx, temp.path()).unwrap();
        let root = temp.path().join("ponte-app");

        // Wrapper oficial com a versão e o checksum de versions.rs.
        let props =
            fs::read_to_string(root.join("android/gradle/wrapper/gradle-wrapper.properties"))
                .unwrap();
        assert!(props.contains(&format!("gradle-{}-bin.zip", versions::GRADLE)));
        assert!(props.contains(&format!(
            "distributionSha256Sum={}",
            versions::GRADLE_DIST_SHA256
        )));
        let jar = fs::read(root.join("android/gradle/wrapper/gradle-wrapper.jar")).unwrap();
        assert_eq!(
            &jar[..2],
            b"PK",
            "gradle-wrapper.jar deve ser um zip válido"
        );

        let script = fs::read_to_string(root.join("scripts/build-android-ffi.sh")).unwrap();
        assert!(script.contains("CRATE=\"ponte-app-ffi\""));
        assert!(script.contains("--language kotlin"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for exe in ["scripts/build-android-ffi.sh", "android/gradlew"] {
                let mode = fs::metadata(root.join(exe)).unwrap().permissions().mode();
                assert_eq!(mode & 0o111, 0o111, "{exe} deve ser executável");
            }
        }

        // O Gradle chama o script antes do build e usa as versões atuais.
        let app = fs::read_to_string(root.join("android/app/build.gradle.kts")).unwrap();
        assert!(app.contains("scripts/build-android-ffi.sh"));
        assert!(app.contains("\"preDebugBuild\" -> dependsOn(buildRustCoreDebug)"));
        assert!(app.contains(&format!("compileSdk = {}", versions::ANDROID_COMPILE_SDK)));
        let catalog = fs::read_to_string(root.join("android/gradle/libs.versions.toml")).unwrap();
        assert!(catalog.contains(&format!("agp = \"{}\"", versions::AGP)));
        assert!(!catalog.contains("type = \"aar\""));

        // A UI chama o núcleo Rust em vez de simular o estado em Kotlin.
        let activity = fs::read_to_string(
            root.join("android/app/src/main/java/com/example/ponte/MainActivity.kt"),
        )
        .unwrap();
        assert!(activity.contains("import com.example.ponte.uniffi.AppEngine"));
        assert!(activity.contains("engine.decrement()"));
    }

    #[test]
    fn test_template_base_snapshot_matches_generated_files() {
        let temp = tempdir().unwrap();
        let ctx = TemplateContext::new("Base App", "com.example.base", vec!["ios".to_string()]);
        Scaffolder::create_project(&ctx, temp.path()).unwrap();
        let root = temp.path().join("base-app");
        let base = root.join(TEMPLATE_BASE_DIR);

        for rel in [
            "Makefile",
            "ios/project.yml",
            "crates/base-app-core/src/lib.rs",
        ] {
            assert_eq!(
                fs::read(root.join(rel)).unwrap(),
                fs::read(base.join(rel)).unwrap(),
                "{rel} difere da base"
            );
        }
        assert!(
            !base.join(TEMPLATE_BASE_DIR).exists(),
            "a base não contém a si mesma"
        );
    }

    #[test]
    fn test_generated_rust_passes_rustfmt_check() {
        if std::process::Command::new("rustfmt")
            .arg("--version")
            .output()
            .is_err()
        {
            return; // sem toolchain Rust no ambiente de teste
        }
        let temp = tempdir().unwrap();
        for (name, template, storage) in [
            ("Fmt Starter", "starter", "sqlite"),
            ("Fmt Vault", "vault", "sqlite"),
        ] {
            let mut ctx = TemplateContext::with_options(
                name,
                "com.example.fmt",
                vec![
                    "android".to_string(),
                    "ios".to_string(),
                    "desktop".to_string(),
                ],
                storage,
                "gradle",
                "0.4.0",
            );
            ctx.app_template = template.to_string();
            Scaffolder::create_project(&ctx, temp.path()).unwrap();
        }
        let rs_files: Vec<_> = walkdir::WalkDir::new(temp.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .filter(|e| !e.path().components().any(|c| c.as_os_str() == ".jirau"))
            .map(|e| e.path().to_path_buf())
            .collect();
        assert!(rs_files.len() >= 10);
        let out = std::process::Command::new("rustfmt")
            .args(["--edition", versions::RUST_EDITION, "--check"])
            .args(&rs_files)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "código Rust gerado não está formatado:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}
