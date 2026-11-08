use crate::config::JirauConfig;
use crate::ui;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub struct ReleaseBuilder;

impl ReleaseBuilder {
    pub fn build(
        root_dir: &Path,
        config: &JirauConfig,
        requested_platform: Option<String>,
        release: bool,
    ) -> Result<()> {
        ui::print_banner();
        let mode = if release { "RELEASE" } else { "DEBUG" };
        ui::print_step(&format!("Iniciando build [{}]", mode));

        let platforms_to_build: Vec<String> = match requested_platform {
            Some(p) => vec![p.to_lowercase()],
            None => config.app.platforms.clone(),
        };

        let slug = config.app.name.to_lowercase().replace(['_', ' '], "-");
        let ffi_pkg = format!("{}-ffi", slug);

        // Android build
        if platforms_to_build.contains(&"android".to_string()) {
            ui::print_step("Compilando binários nativos para Android (multi-ABI)...");
            let jni_out = root_dir.join("android/app/src/main/jniLibs");

            let mut args = vec![
                "ndk",
                "-t",
                "arm64-v8a",
                "-t",
                "x86_64",
                "-o",
                jni_out.to_str().unwrap(),
                "build",
                "--package",
                &ffi_pkg,
            ];

            if release {
                args.push("--release");
            }

            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args(&args)
                .status()
                .with_context(|| "Falha ao executar 'cargo ndk'")?;

            if status.success() {
                ui::print_success(
                    "Bibliotecas nativas Android (.so) geradas com sucesso em jniLibs!",
                );
            } else {
                ui::print_error("Falha na compilação dos binários nativos Android.");
            }
        }

        // iOS build
        if platforms_to_build.contains(&"ios".to_string()) {
            ui::print_step("Compilando binários nativos para iOS...");
            if cfg!(target_os = "macos") {
                let targets = if release {
                    vec!["aarch64-apple-ios", "aarch64-apple-ios-sim"]
                } else {
                    vec!["aarch64-apple-ios-sim"]
                };

                for target in targets {
                    let mut args = vec!["build", "--target", target, "--package", &ffi_pkg];
                    if release {
                        args.push("--release");
                    }
                    let status = Command::new("cargo")
                        .current_dir(root_dir)
                        .args(&args)
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            ui::print_success(&format!("Target iOS {} compilado.", target));
                        }
                        _ => {
                            ui::print_warning(&format!("Falha ao compilar target iOS {}.", target));
                        }
                    }
                }
            } else {
                ui::print_warning("Compilação final de XCFramework iOS requer ambiente macOS.");
            }
        }

        // Desktop build (Rune UI)
        if platforms_to_build.contains(&"desktop".to_string()) {
            ui::print_step("Compilando binário do aplicativo Desktop (Rune UI)...");
            let desktop_pkg = format!("{}-desktop", slug);

            let mut args = vec!["build", "--package", &desktop_pkg];
            if release {
                args.push("--release");
            }

            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args(&args)
                .status()
                .with_context(|| format!("Falha ao compilar {}", desktop_pkg))?;

            if status.success() {
                let bin_path = if release {
                    format!("target/release/{}", desktop_pkg)
                } else {
                    format!("target/debug/{}", desktop_pkg)
                };
                ui::print_success(&format!(
                    "Binário Desktop gerado com sucesso em: {}",
                    bin_path
                ));
            } else {
                ui::print_error("Falha na compilação do binário Desktop.");
            }
        }

        ui::print_success("Processo de build finalizado.");
        Ok(())
    }
}
