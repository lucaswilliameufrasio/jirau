use crate::config::JirauConfig;
use crate::ui;
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct AndroidDevice {
    pub serial: String,
    pub model: String,
    #[allow(dead_code)]
    pub product: String,
}

#[derive(Debug, Clone)]
pub struct IosSimulator {
    pub name: String,
    pub udid: String,
    pub state: String,
}

pub struct RunEngine;

impl RunEngine {
    pub fn run(
        root_dir: &Path,
        config: &JirauConfig,
        requested_platform: Option<String>,
        device_override: Option<String>,
        watch: bool,
    ) -> Result<()> {
        ui::print_banner();

        // 1. Determina a plataforma
        let platform = match requested_platform {
            Some(p) => p.to_lowercase(),
            None => Self::select_platform(config)?,
        };

        match platform.as_str() {
            "android" => Self::run_android(root_dir, config, device_override.as_deref(), watch),
            "ios" => Self::run_ios(root_dir, config, device_override.as_deref(), watch),
            "desktop" => Self::run_desktop(root_dir, config, watch),
            other => anyhow::bail!(
                "Plataforma '{}' não reconhecida. Use 'android', 'ios' ou 'desktop'.",
                other
            ),
        }
    }

    fn select_platform(config: &JirauConfig) -> Result<String> {
        let options = &config.app.platforms;
        if options.is_empty() {
            return Ok("android".to_string());
        }
        if options.len() == 1 {
            return Ok(options[0].clone());
        }

        let ans = inquire::Select::new("Selecione a plataforma para execução:", options.clone())
            .with_help_message("Use as setas para escolher e Enter para confirmar")
            .prompt();

        match ans {
            Ok(choice) => Ok(choice),
            Err(_) => anyhow::bail!("Operação cancelada pelo usuário."),
        }
    }

    // =========================================================================
    // Android Execution
    // =========================================================================

    pub fn list_android_devices() -> Vec<AndroidDevice> {
        let out = match Command::new("adb").args(["devices", "-l"]).output() {
            Ok(o) if o.status.success() => o.stdout,
            _ => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&out);
        let mut devices = Vec::new();

        for line in stdout.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                let serial = parts[0].to_string();
                let mut model = serial.clone();
                let mut product = String::new();

                for prop in &parts[2..] {
                    if let Some(m) = prop.strip_prefix("model:") {
                        model = m.replace('_', " ");
                    } else if let Some(p) = prop.strip_prefix("product:") {
                        product = p.to_string();
                    }
                }

                devices.push(AndroidDevice {
                    serial,
                    model,
                    product,
                });
            }
        }

        devices
    }

    pub fn select_or_detect_android_device(device_override: Option<&str>) -> Result<AndroidDevice> {
        let devices = Self::list_android_devices();

        if let Some(target) = device_override {
            if let Some(found) = devices
                .iter()
                .find(|d| d.serial == target || d.model == target)
            {
                return Ok(found.clone());
            }
            return Ok(AndroidDevice {
                serial: target.to_string(),
                model: target.to_string(),
                product: String::new(),
            });
        }

        match devices.len() {
            0 => {
                anyhow::bail!(
                    "Nenhum dispositivo ou emulador Android conectado foi encontrado via ADB.\n\
                     Soluções:\n\
                     • Conecte um aparelho físico via USB com 'Depuração USB' habilitada.\n\
                     • Ou inicie um emulador Android via Android Studio / 'emulator @<nome>'.\n\
                     • Em seguida, execute 'adb devices' para confirmar a conexão."
                );
            }
            1 => {
                let dev = &devices[0];
                ui::print_info(&format!(
                    "Dispositivo Android único detectado: {} ({}) — iniciando diretamente...",
                    dev.model.cyan().bold(),
                    dev.serial.yellow()
                ));
                Ok(dev.clone())
            }
            _ => {
                let items: Vec<String> = devices
                    .iter()
                    .map(|d| format!("{} ({})", d.model, d.serial))
                    .collect();

                let ans =
                    inquire::Select::new("Selecione o dispositivo Android para executar:", items)
                        .with_help_message("Mais de um dispositivo conectado foi detectado")
                        .prompt()
                        .map_err(|_| anyhow::anyhow!("Seleção cancelada pelo usuário."))?;

                let chosen_idx = devices
                    .iter()
                    .position(|d| format!("{} ({})", d.model, d.serial) == ans)
                    .unwrap_or(0);

                Ok(devices[chosen_idx].clone())
            }
        }
    }

    fn run_android(
        root_dir: &Path,
        config: &JirauConfig,
        device_override: Option<&str>,
        watch: bool,
    ) -> Result<()> {
        let dev = Self::select_or_detect_android_device(device_override)?;

        // Detecta ABI do dispositivo selecionado
        let abi = Command::new("adb")
            .args(["-s", &dev.serial, "shell", "getprop", "ro.product.cpu.abi"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "arm64-v8a".to_string());

        let abi = if abi.is_empty() {
            "arm64-v8a".to_string()
        } else {
            abi
        };

        ui::print_step(&format!(
            "Dispositivo alvo: {} [{}] (ABI: {})",
            dev.model.cyan().bold(),
            dev.serial.yellow(),
            abi.green()
        ));

        // 1. Compilação do Rust Core via cargo-ndk
        let ffi_pkg = format!(
            "{}-ffi",
            config.app.name.to_lowercase().replace(['_', ' '], "-")
        );
        let jni_out = root_dir.join("android/app/src/main/jniLibs");

        let spinner = ui::create_spinner(&format!("Compilando bindings FFI para ABI {}...", abi));

        let status = Command::new("cargo")
            .current_dir(root_dir)
            .args([
                "ndk",
                "-t",
                &abi,
                "-o",
                jni_out.to_str().unwrap(),
                "build",
                "-p",
                &ffi_pkg,
            ])
            .status();

        match status {
            Ok(s) if s.success() => {
                ui::finish_spinner_success(&spinner, "Bindings FFI compilados com sucesso!");
            }
            _ => {
                ui::finish_spinner_error(&spinner, "Falha na compilação via cargo-ndk.");
                anyhow::bail!("Falha ao compilar nativo Android.");
            }
        }

        // 2. Build & Install via Gradle
        let android_dir = root_dir.join("android");
        let gradlew = if cfg!(windows) {
            android_dir.join("gradlew.bat")
        } else {
            android_dir.join("gradlew")
        };

        if !gradlew.exists() {
            ui::print_warning(
                "Gradle Wrapper não encontrado em android/gradlew. Gerando APK via gradle...",
            );
        }

        let gradle_cmd = if gradlew.exists() {
            gradlew.to_str().unwrap().to_string()
        } else {
            "gradle".to_string()
        };

        let spinner_gradle = ui::create_spinner("Compilando e instalando APK no dispositivo...");
        let gradle_res = Command::new(&gradle_cmd)
            .current_dir(&android_dir)
            .args([
                "installDebug",
                "-Pandroid.injected.device.serial",
                &dev.serial,
            ])
            .status();

        match gradle_res {
            Ok(s) if s.success() => {
                ui::finish_spinner_success(&spinner_gradle, "APK instalado com sucesso!");
            }
            _ => {
                ui::finish_spinner_error(
                    &spinner_gradle,
                    "Falha na execução do Gradle installDebug.",
                );
                ui::print_info("Dica: você pode abrir a pasta 'android/' no Android Studio para instalar visualmente.");
                return Ok(());
            }
        }

        // 3. Inicia a MainActivity no dispositivo
        let component = format!("{}/.MainActivity", config.app.package_id);
        ui::print_step(&format!("Iniciando atividade: {}", component.cyan()));

        let _ = Command::new("adb")
            .args(["-s", &dev.serial, "shell", "am", "start", "-n", &component])
            .status();

        ui::print_success("Aplicativo iniciado com sucesso no dispositivo Android!");

        if watch {
            crate::dev::DevRunner::run(root_dir, config, Some("android".to_string()), true)?;
        }

        Ok(())
    }

    // =========================================================================
    // iOS Execution
    // =========================================================================

    pub fn list_ios_simulators() -> Vec<IosSimulator> {
        if !cfg!(target_os = "macos") {
            return Vec::new();
        }

        let out = match Command::new("xcrun")
            .args(["simctl", "list", "devices", "available"])
            .output()
        {
            Ok(o) if o.status.success() => o.stdout,
            _ => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&out);
        let mut sims = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("--") || line.is_empty() {
                continue;
            }

            if line.contains("(") && line.contains(")") {
                if let (Some(start_paren), Some(end_paren)) = (line.find('('), line.rfind(')')) {
                    let name = line[..start_paren].trim().to_string();
                    let rest = &line[start_paren + 1..end_paren];
                    let parts: Vec<&str> = rest.split(") (").collect();

                    if !parts.is_empty() {
                        let udid = parts[0].to_string();
                        let state = if parts.len() > 1 {
                            parts[1].to_string()
                        } else {
                            "Shutdown".to_string()
                        };

                        if name.starts_with("iPhone") || name.starts_with("iPad") {
                            sims.push(IosSimulator { name, udid, state });
                        }
                    }
                }
            }
        }

        sims
    }

    pub fn select_or_detect_ios_simulator(device_override: Option<&str>) -> Result<IosSimulator> {
        if !cfg!(target_os = "macos") {
            anyhow::bail!(
                "Execução em simulador iOS requer macOS com Xcode e xcrun instalados.\n\
                 No Linux, você pode testar o app em 'android' ou 'desktop'."
            );
        }

        let sims = Self::list_ios_simulators();

        if let Some(target) = device_override {
            if let Some(found) = sims.iter().find(|s| s.udid == target || s.name == target) {
                return Ok(found.clone());
            }
            return Ok(IosSimulator {
                name: target.to_string(),
                udid: target.to_string(),
                state: "Unknown".to_string(),
            });
        }

        let booted: Vec<_> = sims
            .iter()
            .filter(|s| s.state.eq_ignore_ascii_case("booted"))
            .cloned()
            .collect();

        match booted.len() {
            1 => {
                let sim = &booted[0];
                ui::print_info(&format!(
                    "Simulador iOS ativo detectado: {} ({}) — iniciando diretamente...",
                    sim.name.cyan().bold(),
                    sim.udid.yellow()
                ));
                Ok(sim.clone())
            }
            n if n > 1 => {
                let items: Vec<String> = booted
                    .iter()
                    .map(|s| format!("{} (Booted - {})", s.name, s.udid))
                    .collect();

                let ans = inquire::Select::new("Selecione o simulador iOS ativo:", items)
                    .prompt()
                    .map_err(|_| anyhow::anyhow!("Seleção cancelada pelo usuário."))?;

                let chosen_idx = booted
                    .iter()
                    .position(|s| format!("{} (Booted - {})", s.name, s.udid) == ans)
                    .unwrap_or(0);

                Ok(booted[chosen_idx].clone())
            }
            _ => {
                // Nenhum simulador booted, exibe a lista de simuladores disponíveis
                if sims.is_empty() {
                    anyhow::bail!(
                        "Nenhum simulador iOS disponível encontrado via 'xcrun simctl list'."
                    );
                }

                let items: Vec<String> = sims
                    .iter()
                    .map(|s| format!("{} ({})", s.name, s.state))
                    .collect();

                let ans = inquire::Select::new(
                    "Nenhum simulador ativo. Selecione um para iniciar:",
                    items,
                )
                .prompt()
                .map_err(|_| anyhow::anyhow!("Seleção cancelada pelo usuário."))?;

                let chosen_idx = sims
                    .iter()
                    .position(|s| format!("{} ({})", s.name, s.state) == ans)
                    .unwrap_or(0);

                let chosen = sims[chosen_idx].clone();

                ui::print_step(&format!(
                    "Inicializando simulador iOS: {}",
                    chosen.name.cyan()
                ));
                let _ = Command::new("xcrun")
                    .args(["simctl", "boot", &chosen.udid])
                    .status();
                let _ = Command::new("open").args(["-a", "Simulator"]).status();

                Ok(chosen)
            }
        }
    }

    fn run_ios(
        root_dir: &Path,
        config: &JirauConfig,
        device_override: Option<&str>,
        watch: bool,
    ) -> Result<()> {
        let sim = Self::select_or_detect_ios_simulator(device_override)?;

        ui::print_step(&format!(
            "Simulador iOS selecionado: {} ({})",
            sim.name.cyan().bold(),
            sim.udid.yellow()
        ));

        // 1. Gera o projeto Xcode se XcodeGen estiver presente
        let ios_dir = root_dir.join("ios");
        if which::which("xcodegen").is_ok() && ios_dir.join("project.yml").exists() {
            let spinner = ui::create_spinner("Regerando projeto Xcode via XcodeGen...");
            let _ = Command::new("xcodegen")
                .current_dir(&ios_dir)
                .args(["generate"])
                .status();
            ui::finish_spinner_success(&spinner, "Projeto Xcode atualizado!");
        }

        // 2. Compilação do Rust Core para o simulador (aarch64-apple-ios-sim)
        let ffi_pkg = format!(
            "{}-ffi",
            config.app.name.to_lowercase().replace(['_', ' '], "-")
        );
        let spinner_rust = ui::create_spinner("Compilando Rust Core para simulador iOS...");
        let rust_status = Command::new("cargo")
            .current_dir(root_dir)
            .args(["build", "-p", &ffi_pkg, "--target", "aarch64-apple-ios-sim"])
            .status();

        match rust_status {
            Ok(s) if s.success() => {
                ui::finish_spinner_success(
                    &spinner_rust,
                    "Rust Core compilado para iOS Simulator!",
                );
            }
            _ => {
                ui::finish_spinner_error(&spinner_rust, "Falha ao compilar crate FFI para iOS.");
            }
        }

        // 3. Execução no simulador
        let project_name = config
            .app
            .name
            .split(['-', '_', ' '])
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>();

        let xcode_proj = ios_dir.join(format!("{}.xcodeproj", project_name));

        if xcode_proj.exists() {
            let spinner_build =
                ui::create_spinner("Compilando e instalando app no simulador iOS...");
            let destination = format!("platform=iOS Simulator,id={}", sim.udid);
            let xcode_status = Command::new("xcodebuild")
                .current_dir(&ios_dir)
                .args([
                    "-project",
                    xcode_proj.to_str().unwrap(),
                    "-scheme",
                    &project_name,
                    "-configuration",
                    "Debug",
                    "-destination",
                    &destination,
                    "build",
                    "CODE_SIGNING_ALLOWED=NO",
                ])
                .status();

            if let Ok(s) = xcode_status {
                if s.success() {
                    ui::finish_spinner_success(&spinner_build, "App iOS compilado com sucesso!");
                } else {
                    ui::finish_spinner_warning(
                        &spinner_build,
                        "xcodebuild retornou aviso. Verifique no Xcode.",
                    );
                }
            }
        } else {
            ui::print_info(
                "Abra o diretório 'ios/' no Xcode para rodar diretamente com debugger gráfico.",
            );
        }

        if watch {
            crate::dev::DevRunner::run(root_dir, config, Some("ios".to_string()), true)?;
        }

        Ok(())
    }

    // =========================================================================
    // Desktop Execution (Rune UI)
    // =========================================================================

    fn run_desktop(root_dir: &Path, config: &JirauConfig, watch: bool) -> Result<()> {
        let pkg_name = format!(
            "{}-desktop",
            config.app.name.to_lowercase().replace(['_', ' '], "-")
        );

        ui::print_step(&format!(
            "Iniciando binário Desktop nativo: {}",
            pkg_name.cyan().bold()
        ));

        if watch {
            crate::dev::DevRunner::run(root_dir, config, Some("desktop".to_string()), true)?;
        } else {
            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args(["run", "--package", &pkg_name])
                .status()
                .with_context(|| format!("Falha ao executar {}", pkg_name))?;

            if !status.success() {
                anyhow::bail!("Execução do binário desktop encerrou com erro.");
            }
        }

        Ok(())
    }
}
