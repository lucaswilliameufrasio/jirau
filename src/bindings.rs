use crate::config::JirauConfig;
use crate::ui;
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct BindingsGenerator;

impl BindingsGenerator {
    pub fn generate(
        root_dir: &Path,
        config: &JirauConfig,
        requested_platform: Option<String>,
    ) -> Result<()> {
        ui::print_banner();
        ui::print_step("Regerando bindings UniFFI para Kotlin e Swift...");

        let slug = config.app.name.to_lowercase().replace(['_', ' '], "-");
        let ffi_crate_dir = root_dir.join("crates").join(format!("{}-ffi", slug));
        let udl_or_lib = ffi_crate_dir.join("src/lib.rs");

        if !udl_or_lib.exists() {
            anyhow::bail!(
                "Não foi possível encontrar a definição FFI em {}",
                udl_or_lib.display()
            );
        }

        let platforms_to_gen: Vec<String> = match requested_platform {
            Some(p) => vec![p.to_lowercase()],
            None => config.app.platforms.clone(),
        };

        // Android Kotlin bindings
        if platforms_to_gen.contains(&"android".to_string()) {
            let out_kotlin = root_dir.join("android/app/build/generated/uniffi");
            fs::create_dir_all(&out_kotlin)?;

            let spinner = ui::create_spinner("Gerando bindings Kotlin...");

            // Invoke cargo uniffi-bindgen or uniffi cli
            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args([
                    "run",
                    "--package",
                    &format!("{}-ffi", slug),
                    "--features",
                    "uniffi/cli",
                    "--bin",
                    "uniffi-bindgen",
                    "--",
                    "generate",
                    udl_or_lib.to_str().unwrap(),
                    "--language",
                    "kotlin",
                    "--out-dir",
                    out_kotlin.to_str().unwrap(),
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    ui::finish_spinner_success(
                        &spinner,
                        &format!("Bindings Kotlin gerados em {}", out_kotlin.display()),
                    );
                }
                _ => {
                    ui::finish_spinner_error(
                        &spinner,
                        "Aviso: bindgen direto não concluiu (será gerado no preBuild do Gradle).",
                    );
                }
            }
        }

        // iOS Swift bindings
        if platforms_to_gen.contains(&"ios".to_string()) {
            let out_swift = root_dir.join("ios/Generated");
            fs::create_dir_all(&out_swift)?;

            let spinner = ui::create_spinner("Gerando bindings Swift...");
            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args([
                    "run",
                    "--package",
                    &format!("{}-ffi", slug),
                    "--features",
                    "uniffi/cli",
                    "--bin",
                    "uniffi-bindgen",
                    "--",
                    "generate",
                    udl_or_lib.to_str().unwrap(),
                    "--language",
                    "swift",
                    "--out-dir",
                    out_swift.to_str().unwrap(),
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    ui::finish_spinner_success(
                        &spinner,
                        &format!("Bindings Swift gerados em {}", out_swift.display()),
                    );
                }
                _ => {
                    ui::finish_spinner_error(
                        &spinner,
                        "Aviso: bindgen direto para Swift requer binário compilado.",
                    );
                }
            }
        }

        ui::print_success("Processamento de bindings finalizado.");
        Ok(())
    }
}
