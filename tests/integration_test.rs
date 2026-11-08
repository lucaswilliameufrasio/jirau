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
