use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_generated_project_cargo_test() {
    let temp = tempdir().unwrap();

    // Compile and run jirau new in the tempdir
    let jirau_bin = env!("CARGO_BIN_EXE_jirau");

    let status = Command::new(jirau_bin)
        .current_dir(temp.path())
        .args([
            "new",
            "test-app",
            "--package",
            "com.test.app",
            "--non-interactive",
        ])
        .status()
        .expect("Falha ao invocar jirau new");

    assert!(status.success(), "jirau new deve ter sucesso");

    let app_dir = temp.path().join("test-app");
    assert!(app_dir.join("Cargo.toml").exists());
    assert!(app_dir.join("jirau.toml").exists());

    // Test the generated core directly with cargo test inside the core crate
    let core_dir = app_dir.join("crates/test-app-core");
    let test_status = Command::new("cargo")
        .current_dir(&core_dir)
        .args(["test", "--offline"])
        .status()
        .expect("Falha ao rodar cargo test na crate core do projeto gerado");

    assert!(
        test_status.success(),
        "cargo test na crate core gerada deve passar"
    );
}
