use crate::config::JirauConfig;
use crate::ui;
use anyhow::{Context, Result};
use colored::*;
use rust_i18n::t;
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
            other => anyhow::bail!("{}", t!("run.unknown_platform", platform = other)),
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

        let ans = inquire::Select::new(&t!("run.select_platform"), options.clone())
            .with_help_message(&t!("run.select_help"))
            .prompt();

        match ans {
            Ok(choice) => Ok(choice),
            Err(_) => anyhow::bail!("{}", t!("run.cancelled")),
        }
    }

    // =========================================================================
    // Android Execution
    // =========================================================================

    pub fn list_android_devices() -> Vec<AndroidDevice> {
        let out = match Command::new(crate::android_pipeline::adb())
            .args(["devices", "-l"])
            .output()
        {
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
            let connected = if devices.is_empty() {
                t!("run.none_connected").into_owned()
            } else {
                devices
                    .iter()
                    .map(|d| format!("{} ({})", d.model, d.serial))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            anyhow::bail!(
                "{}",
                t!(
                    "run.android_device_not_found",
                    target = target,
                    connected = connected
                )
            );
        }

        match devices.len() {
            0 => {
                anyhow::bail!("{}", t!("run.no_android_devices"));
            }
            1 => {
                let dev = &devices[0];
                ui::print_info(&t!(
                    "run.single_android_device",
                    model = dev.model.cyan().bold(),
                    serial = dev.serial.yellow()
                ));
                Ok(dev.clone())
            }
            _ => {
                let items: Vec<String> = devices
                    .iter()
                    .map(|d| format!("{} ({})", d.model, d.serial))
                    .collect();

                let ans = inquire::Select::new(&t!("run.select_android_device"), items)
                    .with_help_message(&t!("run.multiple_devices_help"))
                    .prompt()
                    .map_err(|_| anyhow::anyhow!("{}", t!("run.selection_cancelled")))?;

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
        crate::android_pipeline::deploy_to_device(root_dir, config, &dev)?;
        if watch {
            crate::android_pipeline::watch_and_redeploy(root_dir, config, &dev)?;
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
            .args(["simctl", "list", "devices", "available", "-j"])
            .output()
        {
            Ok(o) if o.status.success() => o.stdout,
            _ => return Vec::new(),
        };

        Self::parse_simctl_devices_json(&out)
    }

    /// Interpreta `xcrun simctl list devices available -j`. O formato JSON evita
    /// ambiguidades de nomes com parênteses (ex.: "iPad Pro 11-inch (M5)").
    pub fn parse_simctl_devices_json(json: &[u8]) -> Vec<IosSimulator> {
        let value: serde_json::Value = match serde_json::from_slice(json) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let mut runtimes: Vec<(&String, &serde_json::Value)> = match value["devices"].as_object() {
            Some(map) => map.iter().filter(|(rt, _)| rt.contains("iOS")).collect(),
            None => return Vec::new(),
        };
        // Runtimes mais novos primeiro.
        runtimes.sort_by(|a, b| b.0.cmp(a.0));

        let mut sims = Vec::new();
        for (_, devices) in runtimes {
            for d in devices.as_array().into_iter().flatten() {
                let (Some(name), Some(udid), Some(state)) =
                    (d["name"].as_str(), d["udid"].as_str(), d["state"].as_str())
                else {
                    continue;
                };
                if name.starts_with("iPhone") || name.starts_with("iPad") {
                    sims.push(IosSimulator {
                        name: name.to_string(),
                        udid: udid.to_string(),
                        state: state.to_string(),
                    });
                }
            }
        }
        sims
    }

    pub fn select_or_detect_ios_simulator(device_override: Option<&str>) -> Result<IosSimulator> {
        if !cfg!(target_os = "macos") {
            anyhow::bail!("{}", t!("run.ios_requires_macos"));
        }

        let sims = Self::list_ios_simulators();

        if let Some(target) = device_override {
            // Prefere um simulador já ligado quando há vários com o mesmo nome.
            let found = sims
                .iter()
                .filter(|s| s.udid == target || s.name == target)
                .max_by_key(|s| s.state.eq_ignore_ascii_case("booted"));
            return match found {
                Some(sim) => Ok(sim.clone()),
                None => anyhow::bail!("{}", t!("run.ios_simulator_not_found", target = target)),
            };
        }

        let booted: Vec<_> = sims
            .iter()
            .filter(|s| s.state.eq_ignore_ascii_case("booted"))
            .cloned()
            .collect();

        match booted.len() {
            1 => {
                let sim = &booted[0];
                ui::print_info(&t!(
                    "run.booted_simulator_detected",
                    name = sim.name.cyan().bold(),
                    udid = sim.udid.yellow()
                ));
                Ok(sim.clone())
            }
            n if n > 1 => {
                let items: Vec<String> = booted
                    .iter()
                    .map(|s| format!("{} (Booted - {})", s.name, s.udid))
                    .collect();

                let ans = inquire::Select::new(&t!("run.select_booted_simulator"), items)
                    .prompt()
                    .map_err(|_| anyhow::anyhow!("{}", t!("run.selection_cancelled")))?;

                let chosen_idx = booted
                    .iter()
                    .position(|s| format!("{} (Booted - {})", s.name, s.udid) == ans)
                    .unwrap_or(0);

                Ok(booted[chosen_idx].clone())
            }
            _ => {
                // Nenhum simulador booted, exibe a lista de simuladores disponíveis
                if sims.is_empty() {
                    anyhow::bail!("{}", t!("run.no_ios_simulators"));
                }

                let items: Vec<String> = sims
                    .iter()
                    .map(|s| format!("{} ({})", s.name, s.state))
                    .collect();

                let ans = inquire::Select::new(&t!("run.no_booted_select_one"), items)
                    .prompt()
                    .map_err(|_| anyhow::anyhow!("{}", t!("run.selection_cancelled")))?;

                let chosen_idx = sims
                    .iter()
                    .position(|s| format!("{} ({})", s.name, s.state) == ans)
                    .unwrap_or(0);

                // O boot acontece em ios_pipeline::ensure_booted.
                let chosen = sims[chosen_idx].clone();

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

        ui::print_step(&t!(
            "run.ios_simulator_selected",
            name = sim.name.cyan().bold(),
            udid = sim.udid.yellow()
        ));

        crate::ios_pipeline::deploy_to_simulator(root_dir, config, &sim)?;
        ui::print_success(&t!("run.ios_app_started"));

        if watch {
            crate::ios_pipeline::watch_and_redeploy(root_dir, config, &sim)?;
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

        ui::print_step(&t!(
            "run.desktop_starting",
            package = pkg_name.cyan().bold()
        ));

        if watch {
            crate::dev::DevRunner::run(root_dir, config, Some("desktop".to_string()), true)?;
        } else {
            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args(["run", "--package", &pkg_name])
                .status()
                .with_context(|| t!("run.exec_failed", package = pkg_name).into_owned())?;

            if !status.success() {
                anyhow::bail!("{}", t!("run.desktop_exit_error"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod ios_simulator_tests {
    use super::RunEngine;

    #[test]
    fn parses_simctl_json_including_names_with_parentheses() {
        let json = br#"{"devices":{
            "com.apple.CoreSimulator.SimRuntime.iOS-26-0":[
                {"name":"iPhone 17","udid":"OLD-1","state":"Shutdown","isAvailable":true}],
            "com.apple.CoreSimulator.SimRuntime.iOS-27-2":[
                {"name":"iPad Pro 11-inch (M5)","udid":"AF1A","state":"Shutdown","isAvailable":true},
                {"name":"iPhone 18 Pro","udid":"6315","state":"Booted","isAvailable":true}],
            "com.apple.CoreSimulator.SimRuntime.watchOS-12-0":[
                {"name":"Apple Watch","udid":"W1","state":"Shutdown","isAvailable":true}]
        }}"#;
        let sims = RunEngine::parse_simctl_devices_json(json);
        let names: Vec<&str> = sims.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            ["iPad Pro 11-inch (M5)", "iPhone 18 Pro", "iPhone 17"]
        );
        assert_eq!(sims[0].udid, "AF1A");
        assert_eq!(sims[1].state, "Booted");
    }

    #[test]
    fn invalid_simctl_output_yields_no_simulators() {
        assert!(RunEngine::parse_simctl_devices_json(b"not json").is_empty());
    }
}
