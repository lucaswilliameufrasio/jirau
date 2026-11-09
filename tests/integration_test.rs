use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_generated_project_cargo_test() {
    let temp = tempdir().unwrap();
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    // Scaffold mobile-first project to verify Rust core compilation without external git dependencies
    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "test-app",
            "--package",
            "com.test.app",
            "--android",
            "--ios",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao invocar jirau new");

    assert!(status.success(), "jirau new deve ter sucesso");

    let app_dir = temp.path().join("test-app");
    assert!(app_dir.join("Cargo.toml").exists());
    assert!(app_dir.join(".mise.toml").exists());
    assert!(app_dir.join("jirau.toml").exists());
    assert!(app_dir.join("crates/test-app-core/Cargo.toml").exists());
    assert!(app_dir.join("crates/test-app-core/src/lib.rs").exists());
    assert!(app_dir.join("android").exists());
    assert!(app_dir.join("ios").exists());

    // Test the generated core directly with cargo test inside the core crate.
    let core_dir = app_dir.join("crates/test-app-core");
    let status = Command::new("cargo")
        .current_dir(&core_dir)
        .arg("test")
        .status();

    let success = match status {
        Ok(s) if s.success() => true,
        _ => Command::new("cargo")
            .current_dir(&core_dir)
            .args(["test", "--offline"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
    };

    assert!(success, "cargo test na crate core gerada deve passar");
}

#[test]
fn test_desktop_scaffold_and_add() {
    let temp = tempdir().unwrap();
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    // 1. Create a project with desktop platform directly
    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "desktop-app",
            "--package",
            "com.test.desktop",
            "--desktop",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao invocar jirau new desktop");

    assert!(status.success(), "jirau new --desktop deve ter sucesso");

    let app_dir = temp.path().join("desktop-app");
    let desktop_cargo = app_dir.join("crates/desktop-app-desktop/Cargo.toml");
    let desktop_main = app_dir.join("crates/desktop-app-desktop/src/main.rs");
    assert!(desktop_cargo.exists(), "Cargo.toml do desktop deve existir");
    assert!(desktop_main.exists(), "main.rs do desktop deve existir");

    let cargo_toml_content = std::fs::read_to_string(&desktop_cargo).unwrap();
    assert!(cargo_toml_content.contains("desktop-app-core"));
    assert!(cargo_toml_content.contains("rune"));

    // 2. Test jirau add desktop on a mobile-only project
    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "mobile-only",
            "--package",
            "com.test.mobile",
            "--android",
            "--ios",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao criar mobile-only");
    assert!(status.success());

    let mobile_dir = temp.path().join("mobile-only");
    assert!(!mobile_dir.join("crates/mobile-only-desktop").exists());

    // Run jirau add desktop
    let status = Command::new(jirau_bin)
        .current_dir(&mobile_dir)
        .args(["add", "desktop"])
        .status()
        .expect("Falha ao invocar jirau add desktop");
    assert!(status.success(), "jirau add desktop deve ter sucesso");

    assert!(
        mobile_dir
            .join("crates/mobile-only-desktop/Cargo.toml")
            .exists()
    );
    assert!(
        mobile_dir
            .join("crates/mobile-only-desktop/src/main.rs")
            .exists()
    );

    let root_cargo = std::fs::read_to_string(mobile_dir.join("Cargo.toml")).unwrap();
    assert!(root_cargo.contains("crates/mobile-only-desktop"));

    let jirau_toml = std::fs::read_to_string(mobile_dir.join("jirau.toml")).unwrap();
    assert!(jirau_toml.contains("desktop"));
}

#[test]
fn test_sqlite_scaffold_and_amper_is_rejected() {
    let temp = tempdir().unwrap();
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "sql-app",
            "--package",
            "com.test.sql",
            "--android",
            "--sqlite",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao criar sql-app");
    assert!(status.success(), "jirau new com --sqlite deve ter sucesso");

    let app_dir = temp.path().join("sql-app");
    let core_cargo =
        std::fs::read_to_string(app_dir.join("crates/sql-app-core/Cargo.toml")).unwrap();
    assert!(core_cargo.contains("rusqlite"));
    let core_lib = std::fs::read_to_string(app_dir.join("crates/sql-app-core/src/lib.rs")).unwrap();
    assert!(core_lib.contains("StorageEngine"));
    assert!(app_dir.join("android/gradlew").exists());

    // Amper (plugin Gradle descontinuado) é recusado com explicação, sem gerar nada.
    let out = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "amper-app",
            "--package",
            "com.test.amper",
            "--android",
            "--amper",
            "--non-interactive",
        ])
        .output()
        .expect("Falha ao invocar jirau new --amper");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Amper"));
    assert!(!temp.path().join("amper-app").exists());
}

#[test]
fn test_upgrade_three_way_merge() {
    let temp = tempdir().unwrap();
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    // 1. Scaffold a project
    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "upgrade-test",
            "--package",
            "com.test.upgrade",
            "--android",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao invocar new");
    assert!(status.success());

    let app_dir = temp.path().join("upgrade-test");

    // 2. Simula um projeto gerado por um template antigo: a base
    //    (.jirau/template-base) registra o que aquele template gerou.
    let base_dir = app_dir.join(".jirau/template-base");
    assert!(
        base_dir.join("Makefile").exists(),
        "jirau new deve gravar a base"
    );

    let config_path = app_dir.join("jirau.toml");
    let config_content = std::fs::read_to_string(&config_path).unwrap();
    let current_version = config_content
        .lines()
        .find(|l| l.starts_with("template_version"))
        .unwrap()
        .to_string();
    std::fs::write(
        &config_path,
        config_content.replace(&current_version, "template_version = \"0.1.0\""),
    )
    .unwrap();

    // 2a. Makefile intocado pelo usuário, mas de um template antigo → atualiza.
    let makefile = app_dir.join("Makefile");
    let new_makefile = std::fs::read_to_string(&makefile).unwrap();
    std::fs::write(&makefile, "# Makefile antigo\n").unwrap();
    std::fs::write(base_dir.join("Makefile"), "# Makefile antigo\n").unwrap();

    // 2b. Usuário customizou e o template não mudou o arquivo → preserva.
    let testing_md = app_dir.join("TESTING_STANDARD.md");
    let orig_md = std::fs::read_to_string(&testing_md).unwrap();
    std::fs::write(
        &testing_md,
        format!("{orig_md}\n<!-- Linha customizada pelo desenvolvedor -->\n"),
    )
    .unwrap();

    // 2c. Ambos mudaram partes diferentes → 3-way merge limpo.
    let gitignore = app_dir.join(".gitignore");
    let template_gitignore = std::fs::read_to_string(&gitignore).unwrap();
    let old_template_gitignore = template_gitignore.replace("Thumbs.db\n", "");
    std::fs::write(base_dir.join(".gitignore"), &old_template_gitignore).unwrap();
    std::fs::write(
        &gitignore,
        format!("# segredos locais\n.env\n\n{old_template_gitignore}"),
    )
    .unwrap();

    // 3. Executa o upgrade
    let status = Command::new(jirau_bin)
        .current_dir(&app_dir)
        .arg("upgrade")
        .status()
        .expect("Falha ao invocar jirau upgrade");
    assert!(status.success(), "jirau upgrade deve ter sucesso");

    // 4. Versão atualizada
    let updated_config = std::fs::read_to_string(&config_path).unwrap();
    assert!(updated_config.contains(&current_version));

    assert_eq!(std::fs::read_to_string(&makefile).unwrap(), new_makefile);
    assert!(
        std::fs::read_to_string(&testing_md)
            .unwrap()
            .contains("Linha customizada pelo desenvolvedor")
    );
    let merged = std::fs::read_to_string(&gitignore).unwrap();
    assert!(merged.contains(".env"), "customização do usuário mantida");
    assert!(merged.contains("Thumbs.db"), "mudança do template aplicada");
    assert!(!merged.contains("<<<<<<<"));

    // 5. A base passa a ser o template atual (para o próximo upgrade).
    assert_eq!(
        std::fs::read_to_string(base_dir.join("Makefile")).unwrap(),
        new_makefile
    );
}

#[test]
fn test_ui_theme_and_ffi_lifecycle_tests() {
    let temp = tempdir().unwrap();
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "themed-app",
            "--package",
            "com.test.themed",
            "--android",
            "--ios",
            "--theme",
            "material3-extended",
            "--plugin",
            "logger",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao invocar jirau new");

    assert!(
        status.success(),
        "jirau new com tema e plugin deve ter sucesso"
    );

    let app_dir = temp.path().join("themed-app");

    // Android theme & lifecycle test files
    assert!(
        app_dir
            .join("android/app/src/main/java/com/test/themed/ui/theme/Color.kt")
            .exists()
    );
    assert!(
        app_dir
            .join("android/app/src/main/java/com/test/themed/ui/theme/Theme.kt")
            .exists()
    );
    assert!(
        app_dir
            .join("android/app/src/main/java/com/test/themed/ui/theme/Type.kt")
            .exists()
    );
    assert!(
        app_dir
            .join("android/app/src/androidTest/java/com/test/themed/NativeLifecycleTest.kt")
            .exists()
    );

    // iOS theme & lifecycle test files
    assert!(app_dir.join("ios/App/Sources/Theme.swift").exists());
    assert!(
        app_dir
            .join("ios/App/Tests/NativeLifecycleTests.swift")
            .exists()
    );

    // UniFFI Lifecycle Integration Test
    let ffi_test = app_dir.join("crates/themed-app-ffi/tests/lifecycle_integration_tests.rs");
    assert!(ffi_test.exists());
    let ffi_test_code = std::fs::read_to_string(&ffi_test).unwrap();
    assert!(ffi_test_code.contains("test_native_app_lifecycle_and_state_flow"));
    assert!(ffi_test_code.contains("AppEngine::new()"));

    // Configuration check
    let jirau_toml = std::fs::read_to_string(app_dir.join("jirau.toml")).unwrap();
    assert!(jirau_toml.contains("ui_theme = \"material3-extended\""));
    assert!(jirau_toml.contains("plugins = [\"logger\"]"));
}

#[test]
fn test_vault_template_and_ci_workflow() {
    let temp = tempdir().unwrap();
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "vault-app",
            "--package",
            "com.test.vault",
            "--android",
            "--ios",
            "--desktop",
            "--sqlite",
            "--template",
            "vault",
            "--ci",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao invocar jirau new com template vault");

    assert!(
        status.success(),
        "jirau new com template vault deve ter sucesso"
    );

    let app_dir = temp.path().join("vault-app");

    // CI Workflow
    assert!(app_dir.join(".github/workflows/ci.yml").exists());
    let ci_content = std::fs::read_to_string(app_dir.join(".github/workflows/ci.yml")).unwrap();
    assert!(ci_content.contains("cargo test --workspace"));
    assert!(ci_content.contains("runs-on: ubuntu-latest"));

    // Vault-specific core files
    let core_lib =
        std::fs::read_to_string(app_dir.join("crates/vault-app-core/src/lib.rs")).unwrap();
    assert!(core_lib.contains("VaultEngine"));
    assert!(core_lib.contains("VaultCategory"));
    assert!(core_lib.contains("generate_password"));

    // Vault-specific FFI files
    let ffi_lib = std::fs::read_to_string(app_dir.join("crates/vault-app-ffi/src/lib.rs")).unwrap();
    assert!(ffi_lib.contains("NativeCategory"));
    assert!(ffi_lib.contains("NativeVaultEngine"));

    let ffi_test = std::fs::read_to_string(
        app_dir.join("crates/vault-app-ffi/tests/lifecycle_integration_tests.rs"),
    )
    .unwrap();
    assert!(ffi_test.contains("test_vault_ffi_lifecycle_and_concurrency"));

    // Vault-specific Desktop file
    let desktop_main =
        std::fs::read_to_string(app_dir.join("crates/vault-app-desktop/src/main.rs")).unwrap();
    assert!(desktop_main.contains("Desktop Vault"));
    assert!(desktop_main.contains("DesktopState"));

    // Vault-specific Android file
    let main_activity = std::fs::read_to_string(
        app_dir.join("android/app/src/main/java/com/test/vault/MainActivity.kt"),
    )
    .unwrap();
    assert!(main_activity.contains("VaultScreen"));
    assert!(
        main_activity.contains("vm.loadSampleItems")
            || main_activity.contains("vm::loadSampleItems")
    );
    let vault_core = std::fs::read_to_string(
        app_dir.join("android/app/src/main/java/com/test/vault/VaultCore.kt"),
    )
    .unwrap();
    // O app usa o motor Rust real, com a chave guardada no Android Keystore.
    assert!(vault_core.contains("import com.test.vault.uniffi.NativeVaultEngine"));
    assert!(vault_core.contains("NativeVaultEngine.open(path, key)"));
    assert!(vault_core.contains("AndroidKeyStore"));

    // Vault-specific iOS file
    let content_view =
        std::fs::read_to_string(app_dir.join("ios/App/Sources/ContentView.swift")).unwrap();
    assert!(content_view.contains("VaultViewModel"));
    // A UI usa o motor Rust (SQLite + gerador) em vez de dados fixos em Swift.
    assert!(content_view.contains("NativeVaultEngine.open(path: path, key: key)"));
    assert!(content_view.contains("kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly"));
    assert!(content_view.contains("generateSecurePassword(options:"));
    assert!(content_view.contains("engine.listItems("));

    let ios_tests =
        std::fs::read_to_string(app_dir.join("ios/App/Tests/NativeLifecycleTests.swift")).unwrap();
    assert!(ios_tests.contains("NativeVaultEngine.inMemory()"));

    // jirau.toml configuration check
    let jirau_toml = std::fs::read_to_string(app_dir.join("jirau.toml")).unwrap();
    assert!(jirau_toml.contains("template = \"vault\""));
    assert!(jirau_toml.contains("storage = \"sqlite\""));
}
