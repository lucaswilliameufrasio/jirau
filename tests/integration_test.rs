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

    assert!(mobile_dir
        .join("crates/mobile-only-desktop/Cargo.toml")
        .exists());
    assert!(mobile_dir
        .join("crates/mobile-only-desktop/src/main.rs")
        .exists());

    let root_cargo = std::fs::read_to_string(mobile_dir.join("Cargo.toml")).unwrap();
    assert!(root_cargo.contains("crates/mobile-only-desktop"));

    let jirau_toml = std::fs::read_to_string(mobile_dir.join("jirau.toml")).unwrap();
    assert!(jirau_toml.contains("desktop"));
}

#[test]
fn test_sqlite_and_amper_scaffold() {
    let temp = tempdir().unwrap();
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "sql-amper-app",
            "--package",
            "com.test.sqlamper",
            "--android",
            "--sqlite",
            "--amper",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao criar sql-amper-app");

    assert!(
        status.success(),
        "jirau new com --sqlite e --amper deve ter sucesso"
    );

    let app_dir = temp.path().join("sql-amper-app");

    // Amper verification
    assert!(app_dir.join("android/module.yaml").exists());
    assert!(app_dir.join("android/project.yaml").exists());
    assert!(app_dir.join("android/amper").exists());

    let module_yaml = std::fs::read_to_string(app_dir.join("android/module.yaml")).unwrap();
    assert!(module_yaml.contains("namespace: \"com.test.sqlamper\""));

    // SQLite verification
    let core_cargo =
        std::fs::read_to_string(app_dir.join("crates/sql-amper-app-core/Cargo.toml")).unwrap();
    assert!(core_cargo.contains("rusqlite"));

    let core_lib =
        std::fs::read_to_string(app_dir.join("crates/sql-amper-app-core/src/lib.rs")).unwrap();
    assert!(core_lib.contains("StorageEngine"));
    assert!(core_lib.contains("kv_store"));

    let jirau_toml = std::fs::read_to_string(app_dir.join("jirau.toml")).unwrap();
    assert!(jirau_toml.contains("storage = \"sqlite\""));
    assert!(jirau_toml.contains("android_build_system = \"amper\""));
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

    // 2. Simulate an older template version 0.1.0 in jirau.toml
    let config_path = app_dir.join("jirau.toml");
    let config_content = std::fs::read_to_string(&config_path).unwrap();
    let old_config = config_content.replace(
        "template_version = \"0.3.0\"",
        "template_version = \"0.1.0\"",
    );
    std::fs::write(&config_path, old_config).unwrap();

    // 3. Simulate user modification in TESTING_STANDARD.md
    let testing_md = app_dir.join("TESTING_STANDARD.md");
    let orig_md = std::fs::read_to_string(&testing_md).unwrap();
    let user_md = format!(
        "{}\n<!-- Linha customizada pelo desenvolvedor -->\n",
        orig_md
    );
    std::fs::write(&testing_md, &user_md).unwrap();

    // 4. Run jirau upgrade
    let status = Command::new(jirau_bin)
        .current_dir(&app_dir)
        .arg("upgrade")
        .status()
        .expect("Falha ao invocar jirau upgrade");
    assert!(status.success(), "jirau upgrade deve ter sucesso");

    // 5. Verify template_version was bumped to 0.3.0
    let updated_config = std::fs::read_to_string(&config_path).unwrap();
    assert!(updated_config.contains("template_version = \"0.3.0\""));

    // 6. Verify user's custom line was preserved
    let updated_md = std::fs::read_to_string(&testing_md).unwrap();
    assert!(updated_md.contains("Linha customizada pelo desenvolvedor"));
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
    assert!(app_dir
        .join("android/app/src/main/java/com/test/themed/ui/theme/Color.kt")
        .exists());
    assert!(app_dir
        .join("android/app/src/main/java/com/test/themed/ui/theme/Theme.kt")
        .exists());
    assert!(app_dir
        .join("android/app/src/main/java/com/test/themed/ui/theme/Type.kt")
        .exists());
    assert!(app_dir
        .join("android/app/src/androidTest/java/com/test/themed/NativeLifecycleTest.kt")
        .exists());

    // iOS theme & lifecycle test files
    assert!(app_dir.join("ios/App/Sources/Theme.swift").exists());
    assert!(app_dir
        .join("ios/App/Tests/NativeLifecycleTests.swift")
        .exists());

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
