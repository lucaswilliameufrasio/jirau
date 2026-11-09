use crate::config::JirauConfig;
use crate::templates::TemplateContext;
use crate::ui;
use anyhow::{Context, Result, bail};
use rust_i18n::t;
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
        ui::print_step(&t!("bindings.regenerating"));

        let ctx = TemplateContext::new(&config.app.name, &config.app.package_id, Vec::new());
        let ffi_pkg = format!("{}-ffi", ctx.slug);
        let ffi_crate_dir = root_dir.join("crates").join(&ffi_pkg);

        if !ffi_crate_dir.join("src/lib.rs").exists() {
            bail!(
                "{}",
                t!(
                    "bindings.ffi_not_found",
                    path = ffi_crate_dir.join("src/lib.rs").display()
                )
            );
        }
        if !ffi_crate_dir.join("src/bin/uniffi-bindgen.rs").exists() {
            bail!("{}", t!("bindings.bindgen_missing", pkg = ffi_pkg));
        }

        let platforms_to_gen: Vec<String> = match requested_platform {
            Some(p) => vec![p.to_lowercase()],
            None => config.app.platforms.clone(),
        };

        if platforms_to_gen.contains(&"android".to_string()) {
            // O script também compila as .so (cargo-ndk) que os bindings carregam.
            crate::android_pipeline::build_ffi(root_dir, false, &[])?;
        }

        if platforms_to_gen.contains(&"ios".to_string()) {
            if cfg!(target_os = "macos") {
                // No macOS o script também monta o XCFramework consumido pelo Xcode.
                crate::ios_pipeline::build_ffi(root_dir, false)?;
            } else {
                let out_swift = root_dir.join("ios/Generated");
                Self::generate_library_mode(
                    root_dir,
                    &ffi_pkg,
                    &ctx.rust_name,
                    "swift",
                    &out_swift,
                )?;
                ui::print_success(&t!("bindings.swift_ready", path = out_swift.display()));
            }
        }

        ui::print_success(&t!("bindings.done"));
        Ok(())
    }

    /// UniFFI "library mode": lê os metadados das macros `#[uniffi::export]`
    /// direto da biblioteca compilada para o host.
    fn generate_library_mode(
        root_dir: &Path,
        ffi_pkg: &str,
        rust_name: &str,
        language: &str,
        out_dir: &Path,
    ) -> Result<()> {
        fs::create_dir_all(out_dir)?;

        let spinner = ui::create_spinner(&t!("bindings.generating", language = language));
        let built = Command::new("cargo")
            .current_dir(root_dir)
            .args(["build", "-p", ffi_pkg, "--lib"])
            .status()
            .with_context(|| t!("bindings.cargo_build_spawn_failed").into_owned())?;
        if !built.success() {
            ui::finish_spinner_error(&spinner, &t!("bindings.host_build_failed"));
            bail!("{}", t!("bindings.cargo_build_failed", pkg = ffi_pkg));
        }

        let lib_file = if cfg!(target_os = "macos") {
            format!("lib{}_ffi.dylib", rust_name)
        } else if cfg!(windows) {
            format!("{}_ffi.dll", rust_name)
        } else {
            format!("lib{}_ffi.so", rust_name)
        };
        let library = root_dir.join("target/debug").join(lib_file);

        let status = Command::new("cargo")
            .current_dir(root_dir)
            .args(["run", "-q", "-p", ffi_pkg, "--bin", "uniffi-bindgen", "--"])
            .args(["generate", "--library"])
            .arg(&library)
            .args(["--language", language, "--out-dir"])
            .arg(out_dir)
            .status()
            .with_context(|| t!("bindings.bindgen_spawn_failed").into_owned())?;

        if status.success() {
            ui::finish_spinner_success(&spinner, &t!("bindings.generated", language = language));
            Ok(())
        } else {
            ui::finish_spinner_error(
                &spinner,
                &t!("bindings.generate_failed", language = language),
            );
            bail!("{}", t!("bindings.bindgen_failed", language = language))
        }
    }
}
