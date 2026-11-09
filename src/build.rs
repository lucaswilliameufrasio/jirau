use crate::config::JirauConfig;
use crate::ui;
use anyhow::{Context, Result};
use rust_i18n::t;
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
        ui::print_step(&t!("build.starting", mode = mode));

        let platforms_to_build: Vec<String> = match requested_platform {
            Some(p) => vec![p.to_lowercase()],
            None => config.app.platforms.clone(),
        };

        let slug = config.app.name.to_lowercase().replace(['_', ' '], "-");

        // Android build: APK com o núcleo Rust de todas as ABIs suportadas.
        if platforms_to_build.contains(&"android".to_string()) {
            let apk_dir = crate::android_pipeline::assemble(root_dir, release, &[])?;
            ui::print_success(&t!("build.apk_ready", path = apk_dir.display()));
        }

        // iOS build: núcleo Rust (device + simulador) → XCFramework + bindings Swift
        if platforms_to_build.contains(&"ios".to_string()) {
            ui::print_step(&t!("build.ios_xcframework"));
            crate::ios_pipeline::build_ffi(root_dir, release)?;
            crate::ios_pipeline::generate_xcode_project(root_dir)?;
            ui::print_success(&t!(
                "build.ios_ready",
                lib = slug.replace('-', "_"),
                project = crate::ios_pipeline::IosNames::from_config(config).pascal_name
            ));
        }

        // Desktop build (Rune UI)
        if platforms_to_build.contains(&"desktop".to_string()) {
            ui::print_step(&t!("build.desktop_compiling"));
            let desktop_pkg = format!("{}-desktop", slug);

            let mut args = vec!["build", "--package", &desktop_pkg];
            if release {
                args.push("--release");
            }

            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args(&args)
                .status()
                .with_context(|| t!("build.compile_failed", pkg = desktop_pkg).into_owned())?;

            if status.success() {
                let bin_path = if release {
                    format!("target/release/{}", desktop_pkg)
                } else {
                    format!("target/debug/{}", desktop_pkg)
                };
                ui::print_success(&t!("build.desktop_ready", path = bin_path));
            } else {
                anyhow::bail!("{}", t!("build.desktop_failed"));
            }
        }

        ui::print_success(&t!("build.done"));
        Ok(())
    }
}
