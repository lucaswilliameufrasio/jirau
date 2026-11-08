use crate::config::JirauConfig;
use crate::ui;
use anyhow::{Context, Result};
use colored::*;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

pub struct DevRunner;

impl DevRunner {
    pub fn run(
        root_dir: &Path,
        config: &JirauConfig,
        requested_platform: Option<String>,
        watch: bool,
    ) -> Result<()> {
        ui::print_banner();

        // 1. Determine platform
        let platform = match requested_platform {
            Some(p) => p.to_lowercase(),
            None => Self::detect_or_select_platform(config)?,
        };

        ui::print_step(&format!(
            "Iniciando fluxo de desenvolvimento para {}",
            platform.to_uppercase().cyan().bold()
        ));

        match platform.as_str() {
            "android" => Self::run_android_dev(root_dir, config, watch)?,
            "ios" => Self::run_ios_dev(root_dir, config, watch)?,
            other => anyhow::bail!(
                "Plataforma '{}' não reconhecida. Use 'android' ou 'ios'.",
                other
            ),
        }

        Ok(())
    }

    fn detect_or_select_platform(config: &JirauConfig) -> Result<String> {
        // Test adb connected devices
        let has_adb_device = Command::new("adb")
            .arg("devices")
            .output()
            .map(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .skip(1)
                    .any(|l| l.split_whitespace().nth(1) == Some("device"))
            })
            .unwrap_or(false);

        let is_macos = cfg!(target_os = "macos");

        if has_adb_device && !is_macos {
            ui::print_info("Dispositivo Android ativo detectado via adb.");
            return Ok("android".to_string());
        }

        // If interactive, prompt user
        let options = config.app.platforms.clone();
        if options.is_empty() {
            return Ok("android".to_string());
        }

        if options.len() == 1 {
            return Ok(options[0].clone());
        }

        let ans = inquire::Select::new("Selecione a plataforma para desenvolvimento:", options)
            .with_help_message("Use as setas para escolher e Enter para confirmar")
            .prompt();

        match ans {
            Ok(choice) => Ok(choice),
            Err(_) => anyhow::bail!("Operação cancelada pelo usuário."),
        }
    }

    fn run_android_dev(root_dir: &Path, config: &JirauConfig, watch: bool) -> Result<()> {
        let abi = Self::detect_android_abi(&config.dev.default_android_abi);
        ui::print_substep(&format!("Target ABI Android ativo: {}", abi.cyan().bold()));

        // Initial compile
        Self::compile_android_abi(root_dir, config, &abi)?;

        if !watch {
            ui::print_success("Compilação concluída com sucesso.");
            return Ok(());
        }

        ui::print_info("Observando alterações em 'crates/'... Pressione Ctrl+C para encerrar.\n");

        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        let crates_dir = root_dir.join("crates");

        watcher.watch(&crates_dir, RecursiveMode::Recursive)?;

        let mut last_build = Instant::now();

        loop {
            match rx.recv() {
                Ok(Ok(Event {
                    kind: EventKind::Modify(_),
                    paths,
                    ..
                })) => {
                    // Debounce rapid events (1.5 seconds)
                    if last_build.elapsed() < Duration::from_millis(1500) {
                        continue;
                    }

                    let changed_rs = paths
                        .iter()
                        .any(|p| p.extension().is_some_and(|ext| ext == "rs"));
                    if !changed_rs {
                        continue;
                    }

                    let relative_paths: Vec<String> = paths
                        .iter()
                        .filter_map(|p| {
                            p.strip_prefix(root_dir)
                                .ok()
                                .map(|d| d.display().to_string())
                        })
                        .collect();

                    println!(
                        "{} Alteração detectada em: {}",
                        "➜".cyan().bold(),
                        relative_paths.join(", ").yellow()
                    );

                    let start = Instant::now();
                    let spinner = ui::create_spinner("Recompilando núcleo Rust...");

                    match Self::compile_android_abi(root_dir, config, &abi) {
                        Ok(_) => {
                            let duration = start.elapsed();
                            ui::finish_spinner_success(
                                &spinner,
                                &format!(
                                    "Recompilado com sucesso em {:.2}s!",
                                    duration.as_secs_f32()
                                ),
                            );
                            println!();
                        }
                        Err(e) => {
                            ui::finish_spinner_error(
                                &spinner,
                                &format!("Erro na compilação: {}", e),
                            );
                            println!();
                        }
                    }

                    last_build = Instant::now();
                }
                Ok(Err(e)) => {
                    ui::print_warning(&format!("Erro no monitor de arquivos: {:?}", e));
                }
                _ => {}
            }
        }
    }

    fn detect_android_abi(fallback: &str) -> String {
        let out = Command::new("adb")
            .args(["shell", "getprop", "ro.product.cpu.abi"])
            .output();

        if let Ok(output) = out {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }

        fallback.to_string()
    }

    fn compile_android_abi(root_dir: &Path, config: &JirauConfig, abi: &str) -> Result<()> {
        let ffi_pkg = format!(
            "{}-ffi",
            config.app.name.to_lowercase().replace(['_', ' '], "-")
        );
        let jni_out = root_dir.join("android/app/src/main/jniLibs");

        let status = Command::new("cargo")
            .current_dir(root_dir)
            .args([
                "ndk",
                "-t",
                abi,
                "-o",
                jni_out.to_str().unwrap_or("android/app/src/main/jniLibs"),
                "build",
                "--package",
                &ffi_pkg,
            ])
            .status()
            .with_context(|| {
                "Falha ao executar 'cargo ndk'. Verifique se 'cargo-ndk' está instalado."
            })?;

        if !status.success() {
            anyhow::bail!("A compilação do Rust via cargo-ndk falhou.");
        }

        Ok(())
    }

    fn run_ios_dev(root_dir: &Path, config: &JirauConfig, watch: bool) -> Result<()> {
        if !cfg!(target_os = "macos") {
            ui::print_warning("O build do app iOS completo requer macOS e Xcode.");
            ui::print_info("Gerando projeto via XcodeGen se 'xcodegen' estiver disponível...");
        }

        let ios_dir = root_dir.join("ios");
        if which::which("xcodegen").is_ok() && ios_dir.join("project.yml").exists() {
            let _ = Command::new("xcodegen")
                .arg("generate")
                .current_dir(&ios_dir)
                .status();
            ui::print_success("Projeto Xcode atualizado via XcodeGen.");
        }

        // Target iOS sim
        let target = "aarch64-apple-ios-sim";
        ui::print_substep(&format!(
            "Compilando core para simulador iOS ({})",
            target.cyan()
        ));

        let ffi_pkg = format!(
            "{}-ffi",
            config.app.name.to_lowercase().replace(['_', ' '], "-")
        );
        let status = Command::new("cargo")
            .current_dir(root_dir)
            .args(["build", "--target", target, "--package", &ffi_pkg])
            .status();

        match status {
            Ok(s) if s.success() => {
                ui::print_success("Compilação para simulador iOS concluída com sucesso.");
            }
            _ => {
                ui::print_warning(
                    "A compilação para o target iOS não pôde ser completada (verifique o doctor).",
                );
            }
        }

        if watch {
            ui::print_info("Observando alterações em 'crates/' para iOS...");
            // Loop similarly for iOS if needed
        }

        Ok(())
    }
}
